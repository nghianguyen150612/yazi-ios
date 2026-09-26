//! The iOS system pasteboard, reached through UIKit's `UIPasteboard`.
//!
//! `UIPasteboard` is the only public pasteboard API on iOS: there is no AppKit
//! equivalent, and the Core Foundation pasteboard calls behind `pbcopy` on
//! macOS are not part of the iOS SDK. It needs no `UIApplication` lifecycle, so
//! a jailbreak command-line process can use it, but it does expect an
//! autorelease pool that nothing else on this path provides — hence the
//! explicit pool around every call.
//!
//! Access is best-effort by design. The pasteboard is an auxiliary capability:
//! a locked device, a read prompt the user declines, or a pasteboard holding
//! only an image must all degrade to "no text available" rather than fail the
//! caller or abort Yazi.

use objc2::{rc::autoreleasepool, runtime::AnyClass};
use objc2_foundation::NSString;
use objc2_ui_kit::UIPasteboard;

/// Whether this process can reach the system pasteboard at all.
///
/// Probed through the runtime rather than assumed, so a system that cannot
/// load UIKit costs a capability instead of a panic. The name is
/// `UIPasteboard::NAME`, which the generated binding only exposes as a `&str`.
pub fn available() -> bool { AnyClass::get(c"UIPasteboard").is_some() }

/// The general pasteboard's text, or `None` if it holds none.
///
/// The pasteboard daemon is reached over XPC and the first call in a process
/// can block for a noticeable while, so callers keep it off the UI thread.
pub fn get() -> Option<Vec<u8>> {
	if !available() {
		return None;
	}

	let s = autoreleasepool(|_| unsafe { UIPasteboard::generalPasteboard().string() })?;
	Some(s.to_string().into_bytes())
}

/// Replaces the general pasteboard's contents with `s`.
///
/// Only plain text is published. Yazi's clipboard is raw bytes, so the caller
/// decides how to render them; the in-process mirror always keeps the original.
pub fn set(s: &str) {
	if !available() {
		return;
	}

	autoreleasepool(|_| {
		let s = NSString::from_str(s);
		unsafe { UIPasteboard::generalPasteboard().setString(Some(&s)) };
	});
}
