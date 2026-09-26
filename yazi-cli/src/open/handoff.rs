//! The one place a file is handed to an application.
//!
//! An opener rule runs a command, so the iOS rules in the default preset name
//! `ya open`, and this is what that command does. It is a platform operation,
//! so it lives behind a seam rather than in the opener pipeline: Yazi still
//! picks the rule, still expands it, and still runs it as a background process,
//! and only the operating-system step differs per platform.
//!
//! The handoff always happens on the device Yazi is running on. That is the
//! point of it — the file lives on the iOS filesystem, so opening it must
//! launch an app on the iOS device, in a local terminal and over SSH alike.
//! Nothing here emits a terminal sequence or an OSC request, because sending
//! "open this" to the machine the user is typing into would open the wrong
//! file: the SSH client has never seen it.

use std::path::Path;

use anyhow::{Context, Result, bail};

/// Hands `path` to the application the operating system associates with it.
pub(super) fn open(path: &Path) -> Result<()> {
	// Whether the path names a folder decides which kind of handler the system
	// looks for, and a path that is not there is a mistake worth naming rather
	// than a capability that is missing.
	let is_dir = match path.metadata() {
		Ok(meta) => meta.is_dir(),
		Err(e) => return Err(e).with_context(|| format!("Cannot open {path:?}")),
	};

	native(path, is_dir)
}

#[cfg(target_os = "ios")]
fn native(path: &Path, is_dir: bool) -> Result<()> {
	match yazi_ffi::launch::open(path, is_dir) {
		Some(true) => Ok(()),
		Some(false) => bail!("No application on this device can open {path:?}"),
		None => bail!("This iOS system does not expose its application handoff"),
	}
}

/// Every other platform already has a working opener rule, so this is only
/// reachable when a configuration points an `ya open`-style rule at a system
/// that cannot honor it.
#[cfg(not(target_os = "ios"))]
fn native(_path: &Path, _is_dir: bool) -> Result<()> {
	bail!(
		"Opening files with the system application handler is not supported on {}",
		std::env::consts::OS
	)
}

// --- Tests
#[cfg(test)]
mod tests {
	use std::path::{Path, PathBuf};

	use super::open;

	fn existing() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_owned() }

	/// A path that is not there cannot be handed to anything, and the reason
	/// has to survive into the diagnostic rather than being reported as a
	/// missing system interface.
	#[test]
	fn test_unopenable_path() {
		let missing = existing().join("no-such-file-for-ya-open");
		let e = open(&missing).unwrap_err();

		// The cause is kept as a typed source rather than flattened into the
		// message, so a caller can still tell what went wrong.
		let io = e.downcast_ref::<std::io::Error>().expect("an io error");
		assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
	}

	/// A path that is there is never reported as a path error, whatever the
	/// platform can do with it: a folder and a document both resolve before
	/// the handoff is even attempted.
	#[test]
	fn test_existing_path() {
		for path in [existing(), existing().join("Cargo.toml")] {
			if let Err(e) = open(&path) {
				assert!(e.downcast_ref::<std::io::Error>().is_none(), "{path:?}: {e:#}");
			}
		}
	}

	/// Off iOS there is no handoff at all, and that is a named capability
	/// rather than a failure of the file.
	#[cfg(not(target_os = "ios"))]
	#[test]
	fn test_unsupported_platform() {
		let e = open(&existing()).unwrap_err();
		assert_eq!(
			e.to_string(),
			format!(
				"Opening files with the system application handler is not supported on {}",
				std::env::consts::OS
			)
		);
	}

	/// A path with a space and non-ASCII characters is carried into the
	/// diagnostic verbatim: nothing percent-encodes or rewrites it on the way to
	/// the operating system.
	#[cfg(unix)]
	#[test]
	fn test_path_in_diagnostic() {
		let missing = Path::new("/tmp/a b/文件.png");
		let e = open(missing).unwrap_err();
		assert_eq!(
			format!("{e:#}"),
			"Cannot open \"/tmp/a b/文件.png\": No such file or directory (os error 2)"
		);
	}
}
