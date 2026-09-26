use super::Clipboard;

impl Clipboard {
	pub async fn get(&self) -> Vec<u8> {
		use clipboard_win::get_clipboard_string;

		let result = tokio::task::spawn_blocking(get_clipboard_string);
		if let Ok(Ok(s)) = result.await {
			return s.into_bytes();
		}

		self.mirrored()
	}

	pub async fn set(&self, s: impl AsRef<[u8]>) {
		use clipboard_win::set_clipboard_string;

		let b = s.as_ref().to_owned();
		self.mirror(&b);

		tokio::task::spawn_blocking(move || set_clipboard_string(&String::from_utf8_lossy(&b)))
			.await
			.ok();
	}
}
