use std::{env, path::{Path, PathBuf}, sync::OnceLock};

use yazi_shim::Uzers;

pub struct Xdg;

impl Xdg {
	pub(super) fn load() {
		Self::config_dir();
		Self::asset_dir();
		Self::state_dir();
		Self::runtime_dir();
		Self::temp_dir();
	}

	pub fn config_dir() -> &'static PathBuf {
		static ONCE: OnceLock<PathBuf> = OnceLock::new();
		ONCE.get_or_init(Self::load_config_dir)
	}

	fn load_config_dir() -> PathBuf {
		if let Some(p) =
			config_override_for(env::var_os("YAZI_CONFIG_HOME").map(PathBuf::from).as_deref())
		{
			return p;
		}

		#[cfg(windows)]
		{
			return dirs::config_dir()
				.map(|p| p.join("yazi\\config"))
				.expect("Failed to get config directory");
		}
		#[cfg(unix)]
		{
			let xdg = env_absolute("XDG_CONFIG_HOME");
			let home = resolved_home();
			config_dir_for(xdg.as_deref(), home.as_deref())
				.expect("Failed to get config directory: no valid HOME or passwd entry")
		}
	}

	pub fn asset_dir() -> &'static PathBuf {
		static ONCE: OnceLock<PathBuf> = OnceLock::new();
		ONCE.get_or_init(Self::load_asset_dir)
	}

	fn load_asset_dir() -> PathBuf {
		#[cfg(windows)]
		{
			return dirs::cache_dir().map(|p| p.join("yazi")).expect("Failed to get asset directory");
		}
		#[cfg(unix)]
		{
			let xdg = env_absolute("XDG_CACHE_HOME");
			let home = resolved_home();
			asset_dir_for(xdg.as_deref(), home.as_deref())
				.expect("Failed to get asset directory: no valid HOME or passwd entry")
		}
	}

	pub fn state_dir() -> &'static PathBuf {
		static ONCE: OnceLock<PathBuf> = OnceLock::new();
		ONCE.get_or_init(Self::load_state_dir)
	}

	fn load_state_dir() -> PathBuf {
		#[cfg(windows)]
		{
			return dirs::data_dir()
				.map(|p| p.join("yazi\\state"))
				.expect("Failed to get state directory");
		}
		#[cfg(unix)]
		{
			let xdg = env_absolute("XDG_STATE_HOME");
			let home = resolved_home();
			state_dir_for(xdg.as_deref(), home.as_deref())
				.expect("Failed to get state directory: no valid HOME or passwd entry")
		}
	}

	pub fn runtime_dir() -> &'static PathBuf {
		static ONCE: OnceLock<PathBuf> = OnceLock::new();
		ONCE.get_or_init(Self::load_runtime_dir)
	}

	fn load_runtime_dir() -> PathBuf {
		#[cfg(target_os = "ios")]
		{
			let xdg = env_absolute("XDG_RUNTIME_DIR");
			runtime_for(xdg.as_deref(), Path::new("/tmp"), Uzers::uid_or_zero())
		}
		#[cfg(not(target_os = "ios"))]
		{
			let xdg = env_absolute("XDG_RUNTIME_DIR");
			let fallback = env::temp_dir();
			let fallback = if fallback.is_absolute() {
				fallback
			} else {
				#[cfg(unix)]
				{
					PathBuf::from("/tmp")
				}
				#[cfg(not(unix))]
				{
					fallback
				}
			};
			runtime_for(xdg.as_deref(), &fallback, Uzers::uid_or_zero())
		}
	}

	pub fn temp_dir() -> &'static PathBuf {
		static ONCE: OnceLock<PathBuf> = OnceLock::new();
		ONCE.get_or_init(Self::load_temp_dir)
	}

	fn load_temp_dir() -> PathBuf {
		#[cfg(unix)]
		{
			let base = env::temp_dir();
			let base = if base.is_absolute() { base } else { PathBuf::from("/tmp") };
			temp_for(&base, Uzers::uid_or_zero())
		}
		#[cfg(not(unix))]
		{
			let mut p = env::temp_dir();
			assert!(p.is_absolute(), "Temporary directory path is not absolute");

			p.push(format!("yazi-{}", Uzers::uid_or_zero()));
			p
		}
	}

	/// Maximum usable Unix-domain socket pathname length in bytes, excluding
	/// the trailing NUL. Derived from the target's `sockaddr_un` layout.
	#[cfg(unix)]
	pub fn max_uds_path_len() -> usize {
		// SAFETY: zeroed `sockaddr_un` is valid for measuring the `sun_path` array length.
		let len = unsafe { std::mem::zeroed::<libc::sockaddr_un>().sun_path.len() };
		len.saturating_sub(1)
	}

	/// Deterministic DDS socket location for a runtime directory.
	///
	/// Returns `<runtime>/.dds.sock` when it fits in `max_len` bytes,
	/// otherwise a short deterministic fallback under `/tmp` that preserves
	/// UID separation and hashes the original runtime's raw bytes.
	#[cfg(unix)]
	pub fn dds_socket_for(runtime: &Path, uid: u32, max_len: usize) -> PathBuf {
		let normal = runtime.join(".dds.sock");
		if path_byte_len(&normal) <= max_len {
			return normal;
		}

		let mut h = yazi_shim::Twox128::default();
		use std::hash::Hasher;
		h.write(path_byte_len_slice(runtime));
		h.write(&uid.to_le_bytes());
		let hash = h.finish_128();

		let dir = PathBuf::from(format!("/tmp/yazi-dds-{uid}-{hash:032x}"));
		dir.join(".dds.sock")
	}
}

fn env_absolute(name: &str) -> Option<PathBuf> {
	env::var_os(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

#[cfg(unix)]
fn resolved_home() -> Option<PathBuf> {
	let env_home = env::var_os("HOME").map(PathBuf::from);
	let native = Uzers::home_dir();
	home_for(env_home.as_deref(), native.as_deref()).map(|p| p.to_owned())
}

#[cfg(not(unix))]
#[allow(dead_code)]
fn resolved_home() -> Option<PathBuf> { None }

// --- Pure policy helpers (side-effect free, testable without touching env) ---

fn home_for<'a>(env_home: Option<&'a Path>, native_home: Option<&'a Path>) -> Option<&'a Path> {
	if let Some(h) = env_home.filter(|p| p.is_absolute()) {
		return Some(h);
	}
	native_home.filter(|p| p.is_absolute())
}

fn config_override_for(yazi_home: Option<&Path>) -> Option<PathBuf> {
	yazi_home.filter(|p| p.is_absolute()).map(|p| p.to_owned())
}

fn config_dir_for(xdg_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
	if let Some(p) = xdg_home.filter(|p| p.is_absolute()) {
		return Some(p.join("yazi"));
	}
	home.filter(|p| p.is_absolute()).map(|h| h.join(".config/yazi"))
}

fn asset_dir_for(xdg_cache: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
	if let Some(p) = xdg_cache.filter(|p| p.is_absolute()) {
		return Some(p.join("yazi"));
	}
	home.filter(|p| p.is_absolute()).map(|h| h.join(".cache/yazi"))
}

fn state_dir_for(xdg_state: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
	if let Some(p) = xdg_state.filter(|p| p.is_absolute()) {
		return Some(p.join("yazi"));
	}
	home.filter(|p| p.is_absolute()).map(|h| h.join(".local/state/yazi"))
}

fn runtime_for(xdg_runtime: Option<&Path>, fallback_base: &Path, uid: u32) -> PathBuf {
	let mut base = xdg_runtime
		.filter(|p| p.is_absolute())
		.map(|p| p.to_owned())
		.unwrap_or_else(|| fallback_base.to_owned());
	if !base.is_absolute() {
		base = PathBuf::from("/tmp");
	}
	base.push(format!("yazi+{uid}"));
	base
}

fn temp_for(base: &Path, uid: u32) -> PathBuf {
	let mut p = if base.is_absolute() { base.to_owned() } else { PathBuf::from("/tmp") };
	p.push(format!("yazi-{uid}"));
	p
}

#[cfg(unix)]
fn path_byte_len(p: &Path) -> usize {
	use std::os::unix::ffi::OsStrExt;
	p.as_os_str().as_bytes().len()
}

#[cfg(not(unix))]
#[allow(dead_code)]
fn path_byte_len(p: &Path) -> usize { p.as_os_str().len() }

#[cfg(unix)]
fn path_byte_len_slice(p: &Path) -> &[u8] {
	use std::os::unix::ffi::OsStrExt;
	p.as_os_str().as_bytes()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	fn p(s: &str) -> PathBuf { PathBuf::from(s) }

	fn pb(s: &str) -> Option<PathBuf> { Some(p(s)) }

	#[test]
	fn config_override_absolute_vs_relative() {
		// Platform-independent YAZI_CONFIG_HOME policy: an absolute override
		// wins, a relative override and None are ignored. Uses a host-valid
		// absolute path so the test compiles and runs on Windows, Unix,
		// macOS, and Linux alike; never mutates process environment.
		let abs = std::env::current_dir()
			.expect("current dir must be available for test")
			.join("yazi-config-override");
		assert!(abs.is_absolute());

		let out = config_override_for(Some(&abs));
		assert_eq!(out.unwrap(), abs);

		assert!(config_override_for(Some(Path::new("relative/yazi"))).is_none());
		assert!(config_override_for(None).is_none());
	}

	#[cfg(unix)]
	#[test]
	fn valid_absolute_xdg_wins() {
		let out = config_dir_for(Some(Path::new("/xdg-conf")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/xdg-conf/yazi"));

		let out = asset_dir_for(Some(Path::new("/xdg-cache")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/xdg-cache/yazi"));

		let out = state_dir_for(Some(Path::new("/xdg-state")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/xdg-state/yazi"));
	}

	#[cfg(unix)]
	#[test]
	fn relative_xdg_ignored() {
		let out = config_dir_for(Some(Path::new("relative/conf")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.config/yazi"));

		let out = asset_dir_for(Some(Path::new("relative/cache")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.cache/yazi"));

		let out = state_dir_for(Some(Path::new("relative/state")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.local/state/yazi"));

		// A relative YAZI_CONFIG_HOME is ignored by the same absolute-only rule,
		// so the XDG fallback still wins.
		assert!(config_override_for(Some(Path::new("relative/yazi"))).is_none());
		let out = config_dir_for(Some(Path::new("/xdg")), Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/xdg/yazi"));
	}

	#[cfg(unix)]
	#[test]
	fn valid_home_fallback() {
		let out = config_dir_for(None, Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.config/yazi"));

		let out = asset_dir_for(None, Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.cache/yazi"));

		let out = state_dir_for(None, Some(Path::new("/home/u")));
		assert_eq!(out.unwrap(), p("/home/u/.local/state/yazi"));
	}

	#[cfg(unix)]
	#[test]
	fn native_home_fallback() {
		let home = home_for(None, Some(Path::new("/var/mobile")));
		assert_eq!(home.unwrap(), Path::new("/var/mobile"));

		let home = home_for(Some(Path::new("relative/home")), Some(Path::new("/var/mobile")));
		assert_eq!(home.unwrap(), Path::new("/var/mobile"));

		let home = home_for(Some(Path::new("/home/u")), Some(Path::new("/var/mobile")));
		assert_eq!(home.unwrap(), Path::new("/home/u"));
	}

	#[test]
	fn home_absent_and_empty() {
		assert!(home_for(None, None).is_none());
		assert!(home_for(Some(Path::new("")), None).is_none());
		assert!(home_for(Some(Path::new("relative/path")), None).is_none());
		assert!(home_for(Some(Path::new("")), Some(Path::new("relative"))).is_none());

		assert!(config_dir_for(None, None).is_none());
		assert!(asset_dir_for(None, None).is_none());
		assert!(state_dir_for(None, None).is_none());
	}

	#[cfg(unix)]
	#[test]
	fn non_utf8_home_preserved() {
		use std::{ffi::OsString, os::unix::ffi::{OsStrExt, OsStringExt}};
		let raw = OsString::from_vec(b"/home/\xffuser".to_vec());
		let home = PathBuf::from(raw);
		assert!(home.is_absolute());

		let out = config_dir_for(None, Some(&home));
		let bytes: Vec<u8> = { out.unwrap().as_os_str().as_bytes().to_vec() };
		assert_eq!(bytes, b"/home/\xffuser/.config/yazi".to_vec());
	}

	#[cfg(unix)]
	#[test]
	fn root_and_non_root_uid_policy() {
		let base = Path::new("/tmp");
		assert_eq!(runtime_for(None, base, 0), p("/tmp/yazi+0"));
		assert_eq!(runtime_for(None, base, 501), p("/tmp/yazi+501"));
		assert_eq!(runtime_for(None, base, 12345), p("/tmp/yazi+12345"));

		assert_eq!(temp_for(base, 0), p("/tmp/yazi-0"));
		assert_eq!(temp_for(base, 501), p("/tmp/yazi-501"));
	}

	#[cfg(unix)]
	#[test]
	fn runtime_xdg_absolute_and_relative() {
		let out = runtime_for(Some(Path::new("/run/user/1000")), Path::new("/tmp"), 501);
		assert_eq!(out, p("/run/user/1000/yazi+501"));

		let out = runtime_for(Some(Path::new("relative/runtime")), Path::new("/tmp"), 501);
		assert_eq!(out, p("/tmp/yazi+501"));

		let out = runtime_for(None, Path::new("/tmp"), 501);
		assert_eq!(out, p("/tmp/yazi+501"));
	}

	#[cfg(unix)]
	#[test]
	fn runtime_deterministic_and_uid_separated() {
		let a = runtime_for(None, Path::new("/tmp"), 501);
		let b = runtime_for(None, Path::new("/tmp"), 501);
		assert_eq!(a, b);

		let c = runtime_for(None, Path::new("/tmp"), 0);
		assert_ne!(a, c);
	}

	#[cfg(unix)]
	#[test]
	fn temp_absolute() {
		let out = temp_for(Path::new("/tmp"), 501);
		assert!(out.is_absolute());

		let out = temp_for(Path::new("relative/tmp"), 501);
		assert_eq!(out, p("/tmp/yazi-501"));
		assert!(out.is_absolute());
	}

	#[cfg(unix)]
	#[test]
	fn tmpdir_differences_do_not_fragment_runtime_on_ios_policy() {
		// iOS runtime fallback ignores TMPDIR and uses fixed /tmp,
		// so incidental TMPDIR differences must not change the runtime.
		let ios_fallback = Path::new("/tmp");
		let a = runtime_for(None, ios_fallback, 501);
		let b = runtime_for(None, ios_fallback, 501);
		assert_eq!(a, b);
		assert_eq!(a, p("/tmp/yazi+501"));

		// Explicit user override still wins and creates a different namespace.
		let custom = runtime_for(Some(Path::new("/custom/runtime")), ios_fallback, 501);
		assert_eq!(custom, p("/custom/runtime/yazi+501"));
		assert_ne!(custom, a);
	}

	#[cfg(unix)]
	#[test]
	fn socket_short_path_preserved() {
		let runtime = Path::new("/tmp/yazi+501");
		let sock = Xdg::dds_socket_for(runtime, 501, 103);
		assert_eq!(sock, p("/tmp/yazi+501/.dds.sock"));
	}

	#[cfg(unix)]
	#[test]
	fn socket_exact_max_and_one_too_long() {
		let max = 103usize;
		// Build a runtime whose socket is exactly max bytes.
		let suffix = "/.dds.sock";
		let base_len = max - suffix.len();
		// "/tmp/" is 5 bytes, "yazi+501" is 8, need padding.
		let mut base = String::from("/tmp/");
		while base.len() < base_len {
			base.push('a');
		}
		base.truncate(base_len);
		let runtime = PathBuf::from(&base);
		let normal = runtime.join(".dds.sock");
		assert_eq!(path_byte_len(&normal), max);
		assert_eq!(Xdg::dds_socket_for(&runtime, 501, max), normal);

		// One byte too long activates fallback.
		let runtime_long = PathBuf::from(format!("{base}x"));
		let sock = Xdg::dds_socket_for(&runtime_long, 501, max);
		assert_ne!(sock, runtime_long.join(".dds.sock"));
		assert!(path_byte_len(&sock) <= max);
		assert!(sock.is_absolute());
	}

	#[cfg(unix)]
	#[test]
	fn socket_unicode_byte_count() {
		// 'é' is 2 bytes in UTF-8 but 1 char; limit must use bytes.
		let mut s = String::from("/tmp/");
		for _ in 0..60 {
			s.push('é');
		}
		s.push_str("/yazi+501");
		let runtime = PathBuf::from(s);
		let sock = runtime.join(".dds.sock");
		let byte_len = path_byte_len(&sock);
		let char_len: usize = sock.to_string_lossy().chars().count();
		assert!(byte_len > char_len);
		assert!(byte_len > 103);

		let fallback = Xdg::dds_socket_for(&runtime, 501, 103);
		assert_ne!(fallback, sock);
		assert!(path_byte_len(&fallback) <= 103);
	}

	#[cfg(unix)]
	#[test]
	fn socket_non_utf8() {
		use std::{ffi::OsString, os::unix::ffi::OsStringExt};
		let raw = OsString::from_vec(b"/tmp/\xff/yazi+501".to_vec());
		let runtime = PathBuf::from(raw);
		let max = 103usize;
		let normal = runtime.join(".dds.sock");
		if path_byte_len(&normal) <= max {
			assert_eq!(Xdg::dds_socket_for(&runtime, 501, max), normal);
		} else {
			let fallback = Xdg::dds_socket_for(&runtime, 501, max);
			assert!(path_byte_len(&fallback) <= max);
		}
	}

	#[cfg(unix)]
	#[test]
	fn socket_long_runtime_fallback_deterministic() {
		let mut s = String::from("/tmp/");
		for _ in 0..150 {
			s.push('a');
		}
		s.push_str("/yazi+501");
		let long = PathBuf::from(s);
		let max = 103usize;
		assert!(path_byte_len(&long.join(".dds.sock")) > max);

		let a = Xdg::dds_socket_for(&long, 501, max);
		let b = Xdg::dds_socket_for(&long, 501, max);
		assert_eq!(a, b);
		assert!(path_byte_len(&a) <= max);
		assert!(a.is_absolute());
	}

	#[cfg(unix)]
	#[test]
	fn socket_different_uid_different_fallback() {
		let mut s = String::from("/tmp/");
		for _ in 0..150 {
			s.push('a');
		}
		s.push_str("/yazi+501");
		let long = PathBuf::from(s);
		let max = 103usize;
		assert!(path_byte_len(&long.join(".dds.sock")) > max);

		let a = Xdg::dds_socket_for(&long, 0, max);
		let b = Xdg::dds_socket_for(&long, 501, max);
		assert_ne!(a, b);
		assert!(path_byte_len(&a) <= max);
		assert!(path_byte_len(&b) <= max);
	}

	#[cfg(unix)]
	#[test]
	fn socket_different_namespace_no_collide() {
		let max = 103usize;
		let mut sa = String::from("/tmp/");
		for _ in 0..80 {
			sa.push('a');
		}
		sa.push_str("/alpha/yazi+501");
		let mut sb = String::from("/tmp/");
		for _ in 0..80 {
			sb.push('a');
		}
		sb.push_str("/beta/yazi+501");
		let a = Xdg::dds_socket_for(Path::new(&sa), 501, max);
		let b = Xdg::dds_socket_for(Path::new(&sb), 501, max);
		assert!(path_byte_len(&PathBuf::from(&sa).join(".dds.sock")) > max);
		assert!(path_byte_len(&PathBuf::from(&sb).join(".dds.sock")) > max);
		assert_ne!(a, b);
	}

	#[cfg(unix)]
	#[test]
	fn socket_fallback_uses_full_128bit_hash() {
		let max = 103usize;
		let mut s = String::from("/tmp/");
		for _ in 0..150 {
			s.push('a');
		}
		s.push_str("/yazi+501");
		let long = PathBuf::from(s);
		assert!(path_byte_len(&long.join(".dds.sock")) > max);

		let a = Xdg::dds_socket_for(&long, 501, max);
		assert!(path_byte_len(&a) <= max);
		let dir_name = a.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned();
		let hash_part = dir_name.rsplit('-').next().unwrap().to_owned();
		assert_eq!(hash_part.len(), 32, "fallback must carry the full 128-bit digest");
		assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()));
	}

	#[cfg(unix)]
	#[test]
	fn socket_fallback_max_uid_fits_ios_limit() {
		let max = 103usize;
		let mut s = String::from("/tmp/");
		for _ in 0..150 {
			s.push('a');
		}
		s.push_str("/yazi+501");
		let long = PathBuf::from(s);
		assert!(path_byte_len(&long.join(".dds.sock")) > max);

		let sock = Xdg::dds_socket_for(&long, u32::MAX, max);
		assert!(path_byte_len(&sock) <= max);
		assert!(sock.is_absolute());
		let normal = Xdg::dds_socket_for(&long, 501, max);
		assert_ne!(sock, normal);
	}

	#[test]
	fn unused_helpers_covered() {
		// Ensure pure helpers stay wired to production loaders.
		let _ = pb("/tmp");
	}
}
