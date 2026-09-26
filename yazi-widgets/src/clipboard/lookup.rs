use std::future::Future;

/// Reads the system clipboard, letting the in-process mirror stand in whenever
/// no backend can answer.
///
/// `native` is only awaited when a backend is worth asking, so an SSH session
/// never spawns one. A clipboard read must never fail: the mirror is the
/// value Yazi itself last wrote, so it is always a better answer than nothing.
pub(super) async fn read(
	native: impl Future<Output = Option<Vec<u8>>>,
	mirrored: Vec<u8>,
) -> Vec<u8> {
	// No protocol lets a remote process read its client terminal's OS
	// clipboard, so over SSH the mirror answers and no local backend is
	// spawned at all.
	if yazi_shared::in_ssh_connection() {
		return mirrored;
	}

	native.await.unwrap_or(mirrored)
}

#[cfg(test)]
mod tests {
	use std::sync::{Mutex, MutexGuard};

	use futures::executor::block_on;

	use super::read;

	const SSH_VARS: [&str; 3] = ["SSH_CLIENT", "SSH_TTY", "SSH_CONNECTION"];

	/// `in_ssh_connection()` reads process-wide variables, so every case owns
	/// them exclusively and starts from a known-clean state.
	fn env() -> MutexGuard<'static, ()> {
		static ENV: Mutex<()> = Mutex::new(());

		let guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
		for name in SSH_VARS {
			unsafe { std::env::remove_var(name) };
		}
		guard
	}

	/// A system clipboard that answers wins over the mirror, so text copied in
	/// another app can be pasted into Yazi.
	#[test]
	fn prefers_the_system_clipboard() {
		let _env = env();
		let got = block_on(read(async { Some(b"system".to_vec()) }, b"mirror".to_vec()));
		assert_eq!(got, b"system");
	}

	/// Nothing readable — an empty pasteboard, a locked device, a denied
	/// prompt — falls back instead of reporting an empty paste.
	#[test]
	fn falls_back_when_the_backend_is_silent() {
		let _env = env();
		let got = block_on(read(async { None }, b"mirror".to_vec()));
		assert_eq!(got, b"mirror");
	}

	/// Over SSH the mirror answers without consulting a local backend, because
	/// that backend could only ever describe the device, not the client
	/// terminal the user is actually typing into.
	#[test]
	fn skips_the_backend_over_ssh() {
		let _env = env();
		for name in SSH_VARS {
			unsafe { std::env::set_var(name, "test") };
		}

		let mut asked = false;
		let got = block_on(read(
			async {
				asked = true;
				Some(b"system".to_vec())
			},
			b"mirror".to_vec(),
		));

		assert!(!asked, "the backend should not be asked");
		assert_eq!(got, b"mirror");
	}
}
