//! `ya open` — hand files to the application the system associates with them.
//!
//! The iOS opener rules in the default preset run this, which is the only
//! reason it exists: an opener rule is a command, so the iOS handoff needs a
//! command to name. Everything around it stays upstream — the rule is matched,
//! expanded, and run through the ordinary process path.

use std::{ffi::OsString, path::Path};

use anyhow::Result;

use super::handoff;

pub struct Open;

impl Open {
	/// Hands every target to the system, in the order given.
	///
	/// The preset rules pass one path per invocation with `%s1`, so a selection
	/// of several files arrives as several calls and every one of them is
	/// opened rather than only the first.
	pub(crate) fn run(targets: &[OsString]) -> Result<()> {
		targets.iter().try_for_each(|t| handoff::open(Path::new(t)))
	}
}

// --- Tests
#[cfg(test)]
mod tests {
	use std::ffi::OsString;

	use super::Open;

	/// Targets are handed off one at a time, in order, and a failure names the
	/// target that caused it rather than the whole list.
	#[cfg(unix)]
	#[test]
	fn test_every_target_is_handed_off() {
		let targets: [OsString; 3] =
			["/tmp/one.png", "/tmp/two b.png", "/tmp/三.png"].map(OsString::from);

		let e = Open::run(&targets).unwrap_err();
		assert!(format!("{e:#}").contains("/tmp/one.png"), "{e:#}");

		// An empty list has nothing to hand off, and is not an error.
		assert!(Open::run(&[]).is_ok());
	}

	/// A filename that is not valid UTF-8 reaches the operating system as the
	/// bytes it is. Nothing on the way converts it to a string, so the
	/// diagnostic shows the escaped byte rather than a replacement character.
	#[cfg(unix)]
	#[test]
	fn test_non_utf8_target() {
		use std::os::unix::ffi::OsStringExt;

		let target = OsString::from_vec(b"/tmp/\xff.png".to_vec());
		let e = Open::run(&[target]).unwrap_err();
		assert!(format!("{e:#}").contains("\\xFF"), "{e:#}");
	}
}
