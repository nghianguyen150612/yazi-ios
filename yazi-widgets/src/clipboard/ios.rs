//! The clipboard of a local jailbreak terminal lives in the device's system
//! pasteboard, which UIKit exposes to any process that can reach it.
//!
//! The module is also compiled by host tests so the selection and encoding
//! rules can be exercised; only the UIKit calls need a device.

use std::borrow::Cow;

#[cfg(target_os = "ios")]
use yazi_ffi::pasteboard;

#[cfg(target_os = "ios")]
use super::{Clipboard, lookup};

/// Clipboard bytes as the device pasteboard can carry them.
///
/// `UIPasteboard` is reached through its string API, so the raw bytes Yazi
/// keeps — which may be a filename that is not valid UTF-8 — are converted on
/// the way out. Invalid sequences become U+FFFD, the conversion the Windows
/// backend already used, so pasting never panics and the device clipboard
/// always reflects the latest yank. The in-process mirror keeps the original.
pub(super) fn as_text(s: &[u8]) -> Cow<'_, str> { String::from_utf8_lossy(s) }

#[cfg(target_os = "ios")]
impl Clipboard {
	pub async fn get(&self) -> Vec<u8> {
		lookup::read(
			async {
				// The pasteboard is reached over XPC, and the first call in a
				// process can block for a noticeable while, so it never runs
				// on the thread that drives the UI.
				tokio::task::spawn_blocking(pasteboard::get).await.ok().flatten()
			},
			self.mirrored(),
		)
		.await
	}

	pub async fn set(&self, s: impl AsRef<[u8]>) {
		use yazi_macro::writef;
		use yazi_tty::{TTY, sequence::SetClipboard};

		self.mirror(s.as_ref());
		writef!(TTY.writer(), "{}", SetClipboard(s.as_ref())).ok();

		// A local yank goes to the device pasteboard even over SSH: the
		// terminal emulator that receives OSC 52 runs on the user's PC, so it
		// is the only channel to that PC's clipboard, and it says nothing
		// about the clipboard the rest of the device shares.
		let b = s.as_ref().to_owned();
		tokio::task::spawn_blocking(move || pasteboard::set(&as_text(&b))).await.ok();
	}
}

#[cfg(test)]
mod tests {
	use super::as_text;

	#[test]
	fn keeps_valid_text_untouched() {
		assert_eq!(as_text(b"/tmp/a\tb\nc \xf0\x9f\x97\x82"), "/tmp/a\tb\nc \u{1f5c2}");
		assert_eq!(as_text(b""), "");
	}

	/// A non-UTF-8 filename must degrade rather than panic or drop the yank.
	#[test]
	fn replaces_invalid_sequences() {
		assert_eq!(as_text(b"/tmp/j\xffhn"), "/tmp/j\u{fffd}hn");
	}
}
