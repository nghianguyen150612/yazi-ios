use parking_lot::Mutex;
use yazi_shim::cell::RoCell;

pub static CLIPBOARD: RoCell<Clipboard> = RoCell::new();

/// The running Yazi process' own copy of the clipboard.
///
/// A system clipboard is an auxiliary capability: it may be absent, refused, or
/// hold something other than what Yazi last wrote. The mirror is the one
/// source that is always available, so it records every `set()` verbatim and
/// answers whenever no backend can.
#[derive(Default)]
pub struct Clipboard {
	content: Mutex<Vec<u8>>,
}

impl Clipboard {
	/// Records `s` exactly as given, without any encoding conversion.
	pub(super) fn mirror(&self, s: &[u8]) { s.clone_into(&mut self.content.lock()); }

	/// The last mirrored value, used wherever a system clipboard cannot answer.
	pub(super) fn mirrored(&self) -> Vec<u8> { self.content.lock().clone() }
}

#[cfg(test)]
mod tests {
	use super::Clipboard;

	/// The mirror is a byte-exact record, so a yank of a non-UTF-8 filename is
	/// still pasteable and a backend that needs text cannot corrupt it.
	#[test]
	fn mirrors_raw_bytes() {
		let c = Clipboard::default();
		c.mirror(b"/tmp/j\xffhn\n");
		assert_eq!(c.mirrored(), b"/tmp/j\xffhn\n");
	}

	/// A later yank replaces the earlier one outright, including with nothing.
	#[test]
	fn mirrors_the_latest_value() {
		let c = Clipboard::default();
		c.mirror(b"first");
		c.mirror(b"second");
		assert_eq!(c.mirrored(), b"second");

		c.mirror(b"");
		assert_eq!(c.mirrored(), b"");
	}
}
