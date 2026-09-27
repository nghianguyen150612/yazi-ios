use std::{io, path::PathBuf};

use tokio::{io::{AsyncBufReadExt, BufReader, Lines, ReadHalf, WriteHalf}, sync::OnceCell};
use yazi_fs::{Xdg, create_owned_dir, engine::{Engine, local::Local}};
use yazi_shim::{Uzers, tokio::net::{UnixStream, UnixStreamExt}};

pub struct Stream;

pub type ClientReader = Lines<BufReader<ReadHalf<UnixStream>>>;

pub(super) type ClientWriter = WriteHalf<UnixStream>;

#[cfg(unix)]
pub(super) type ServerListener = tokio::net::UnixListener;
#[cfg(windows)]
pub(super) type ServerListener = WinUnixListener;

impl Stream {
	pub async fn connect() -> io::Result<(ClientReader, ClientWriter)> {
		let stream = UnixStream::connect_uds(Self::socket_file().await?).await?;
		let (reader, writer) = tokio::io::split(stream);
		Ok((BufReader::new(reader).lines(), writer))
	}

	#[cfg(unix)]
	pub(super) async fn bind() -> io::Result<ServerListener> {
		let p = Self::socket_file().await?;

		Local::regular(&p).remove_file().await.ok();
		tokio::net::UnixListener::bind(p)
	}

	#[cfg(windows)]
	pub(super) async fn bind() -> io::Result<ServerListener> {
		let p = Self::socket_file().await?;
		Local::regular(&p).remove_file().await.ok();

		let listener = uds_windows::UnixListener::bind(p)?;
		listener.set_nonblocking(true)?;

		Ok(WinUnixListener(listener))
	}

	async fn socket_file() -> io::Result<&'static PathBuf> {
		static ONCE: OnceCell<PathBuf> = OnceCell::const_new();
		ONCE
			.get_or_try_init(|| async move {
				#[cfg(unix)]
				{
					let runtime = Xdg::runtime_dir();
					let sock =
						Xdg::dds_socket_for(runtime, Uzers::uid_or_zero(), Xdg::max_uds_path_len());
					let parent = sock.parent().unwrap_or(runtime);
					create_owned_dir(parent).await?;

					Ok(sock)
				}
				#[cfg(not(unix))]
				{
					let p = Xdg::runtime_dir();
					create_owned_dir(p).await?;

					Ok(p.join(".dds.sock"))
				}
			})
			.await
	}

	#[cfg(unix)]
	#[allow(dead_code)]
	fn socket_file_for(runtime: &std::path::Path, uid: u32, max_len: usize) -> PathBuf {
		Xdg::dds_socket_for(runtime, uid, max_len)
	}
}

// --- WinUnixListener
#[cfg(windows)]
pub(super) struct WinUnixListener(uds_windows::UnixListener);

#[cfg(windows)]
impl WinUnixListener {
	pub(super) async fn accept(
		&self,
	) -> io::Result<(tokio::net::TcpStream, uds_windows::SocketAddr)> {
		loop {
			match self.0.accept() {
				Ok((stream, addr)) => return Ok((UnixStream::from_uds(stream)?, addr)),
				Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
					tokio::time::sleep(std::time::Duration::from_millis(20)).await;
				}
				Err(e) => return Err(e),
			}
		}
	}
}

#[cfg(all(test, unix))]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn socket_path_short_preserved() {
		let runtime = Path::new("/tmp/yazi+501");
		let sock = Xdg::dds_socket_for(runtime, 501, Xdg::max_uds_path_len());
		assert_eq!(sock, runtime.join(".dds.sock"));
	}

	#[test]
	fn socket_path_long_activates_fallback() {
		let mut s = String::from("/tmp/");
		for _ in 0..150 {
			s.push('a');
		}
		s.push_str("/yazi+501");
		let runtime = PathBuf::from(s);
		let max = Xdg::max_uds_path_len();

		let normal = runtime.join(".dds.sock");
		assert!(path_len(&normal) > max);

		let a = Xdg::dds_socket_for(&runtime, 501, max);
		let b = Xdg::dds_socket_for(&runtime, 501, max);
		assert_eq!(a, b);
		assert!(path_len(&a) <= max);

		let c = Xdg::dds_socket_for(&runtime, 0, max);
		assert_ne!(a, c);
	}

	#[test]
	fn socket_path_uses_bytes_not_chars() {
		let mut s = String::from("/tmp/");
		for _ in 0..60 {
			s.push('é');
		}
		s.push_str("/yazi+501");
		let runtime = PathBuf::from(s);
		let sock = runtime.join(".dds.sock");
		let bytes = path_len(&sock);
		let chars: usize = sock.to_string_lossy().chars().count();
		assert!(bytes > chars);
	}

	#[test]
	fn ios_limit_is_103_from_darwin_layout() {
		// SOURCE-VERIFIED: libc 0.2.186 `unix/bsd/mod.rs` defines
		// `sockaddr_un.sun_path` as `[c_char; 104]` for Darwin/BSD,
		// so usable pathname bytes are 104-1 for the trailing NUL.
		// This host test injects 103 when exercising iOS policy;
		// it does not prove the iOS SDK link, which remains COMPILE-VALIDATED.
		let max = 103usize;
		let runtime = Path::new("/tmp/yazi+501");
		assert_eq!(Xdg::dds_socket_for(runtime, 501, max), runtime.join(".dds.sock"));
	}

	fn path_len(p: &Path) -> usize {
		use std::os::unix::ffi::OsStrExt;
		p.as_os_str().as_bytes().len()
	}

	#[tokio::test]
	async fn uds_bind_connect_and_stale_replace() {
		yazi_shim::init_tests();
		let base = std::env::temp_dir().join(format!(
			"yazi-dds-test-{}",
			std::process::id()
		));
		let _ = tokio::fs::remove_dir_all(&base).await;
		yazi_fs::create_owned_dir(&base).await.expect("create test runtime dir");

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let mode = tokio::fs::metadata(&base).await.unwrap().permissions().mode() & 0o777;
			assert_eq!(mode, 0o700);
		}

		let sock = base.join(".dds.sock");
		let _ = tokio::fs::remove_file(&sock).await;

		// First bind succeeds.
		let listener = tokio::net::UnixListener::bind(&sock).expect("bind test socket");
		// Client connects.
		let client = tokio::net::UnixStream::connect(&sock).await.expect("connect test socket");
		drop(client);
		drop(listener);

		// Stale socket file remains; second bind replaces it after removal.
		assert!(sock.exists());
		tokio::fs::remove_file(&sock).await.ok();
		let listener2 = tokio::net::UnixListener::bind(&sock).expect("rebind after stale cleanup");
		let _client2 = tokio::net::UnixStream::connect(&sock).await.expect("reconnect");
		drop(listener2);
		let _ = tokio::fs::remove_file(&sock).await;
		let _ = tokio::fs::remove_dir_all(&base).await;
	}

	#[tokio::test]
	async fn uds_too_long_path_fails() {
		// Documents the failure mode when a pathname exceeds the OS limit.
		// On Linux the limit is 107 bytes; on Darwin/iOS it is 103.
		// This host test uses an obviously oversized path and expects bind
		// to fail, without claiming it proves the iOS limit.
		let mut s = String::from("/tmp/");
		for _ in 0..200 {
			s.push('b');
		}
		s.push_str("/.dds.sock");
		let result = tokio::net::UnixListener::bind(&s);
		assert!(result.is_err(), "oversized UDS path should fail to bind");
	}
}
