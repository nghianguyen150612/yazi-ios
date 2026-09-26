//! Handing a document to the application iOS associates with it.
//!
//! The public way to do this is `UIApplication`, and that is not an option
//! here: it needs a `UIApplicationMain` lifecycle, a window, and main-thread
//! execution, none of which a command-line process without an app bundle can
//! offer. `UIDocumentInteractionController` and `UIActivityViewController` are
//! the same story — both need a view to present from. What a daemon, an SSH
//! session, and a terminal all reach is LaunchServices'
//! `LSApplicationWorkspace`, a private CoreServices class, which is also what
//! the jailbreak `uiopen` family is built on.
//!
//! It is a private interface, so nothing here is assumed. The framework is
//! opened explicitly, the class is looked up through the Objective-C runtime
//! instead of being linked, and every selector is probed with
//! `respondsToSelector:` before it is sent. What it buys over the other
//! private door into SpringBoard, `SBSOpenSensitiveURLAndUnlock`, is the
//! `BOOL` that comes back: that is what lets "no app on this device handles
//! that" be reported instead of guessed.
//!
//! The URL is built from the path's own bytes through
//! `fileURLWithFileSystemRepresentation:`, so a filename that is not valid
//! UTF-8 reaches the system unchanged rather than through a lossy `NSString`.

use std::{ffi::{CStr, CString}, os::unix::ffi::OsStrExt, path::Path, ptr::NonNull};

use objc2::{msg_send, rc::{Retained, autoreleasepool}, runtime::{AnyClass, AnyObject, Sel}};
use objc2_foundation::NSURL;

/// The frameworks that have published `LSApplicationWorkspace`.
///
/// Both live in the read-only system volume, so this is not a guess about
/// where a jailbreak put anything.
const FRAMEWORKS: [&str; 2] = [
	"/System/Library/Frameworks/MobileCoreServices.framework/MobileCoreServices",
	"/System/Library/Frameworks/CoreServices.framework/CoreServices",
];

/// `-openURL:withOptions:`, the spelling current headers advertise.
const OPEN_WITH_OPTIONS: &CStr = c"openURL:withOptions:";

/// `-openURL:`, the original, and still what older systems answer to.
const OPEN: &CStr = c"openURL:";

/// Hands `path` to the application iOS associates with it.
///
/// `is_dir` marks the URL as a folder, which is what separates a folder
/// handler such as Files from a document handler. The answer is
/// `Some(true)` when a handler accepted it, `Some(false)` when the device has
/// none for it, and `None` when there is no handoff to make at all — a class
/// or selector that is gone, or a path that cannot become a URL. `None` is
/// deliberately not an error; the caller decides what an absent capability
/// means.
pub fn open(path: &Path, is_dir: bool) -> Option<bool> {
	let class = load()?;

	// A real path cannot contain a NUL, so this only rejects a path that never
	// came from the filesystem; treating it as "no handoff" keeps the caller
	// from having to tell it apart from an absent interface.
	let path = CString::new(path.as_os_str().as_bytes()).ok()?;

	autoreleasepool(|_| {
		// SAFETY: `defaultWorkspace` is a singleton; the pool above outlives
		// every object created below it.
		unsafe {
			let workspace: Option<Retained<AnyObject>> = msg_send![class, defaultWorkspace];
			let Some(workspace) = workspace else { return None };

			// SAFETY: a `CString`'s buffer is NUL-terminated, its pointer is
			// never null, and the `CString` it belongs to outlives every call
			// below.
			let path = NonNull::new_unchecked(path.as_ptr().cast_mut());
			let url =
				NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(path, is_dir, None);

			let sel = Sel::register(OPEN_WITH_OPTIONS);
			if responds(&workspace, sel) {
				let taken: bool =
					msg_send![&*workspace, openURL: &*url, withOptions: Option::<&AnyObject>::None];
				return Some(taken);
			}

			let sel = Sel::register(OPEN);
			if responds(&workspace, sel) {
				let taken: bool = msg_send![&*workspace, openURL: &*url];
				return Some(taken);
			}

			// A workspace that answers to neither spelling is one this process
			// cannot drive.
			None
		}
	})
}

/// The `LSApplicationWorkspace` class, once the framework that defines it has
/// been opened.
fn load() -> Option<&'static AnyClass> {
	for framework in FRAMEWORKS {
		let Ok(name) = CString::new(framework) else { continue };

		// SAFETY: a system framework path, opened lazily and never unloaded.
		// Opening it registers the class through the Objective-C runtime.
		unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };

		if let Some(class) = AnyClass::get(c"LSApplicationWorkspace") {
			return Some(class);
		}
	}

	None
}

/// SAFETY: `object` must be an Objective-C object, and `sel` must take no
/// argument and return `BOOL`, which `respondsToSelector:` does.
unsafe fn responds(object: &AnyObject, sel: Sel) -> bool {
	// SAFETY: the caller guarantees the selector's shape.
	unsafe { msg_send![object, respondsToSelector: sel] }
}
