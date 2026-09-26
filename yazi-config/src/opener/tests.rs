use std::sync::OnceLock;

use hashbrown::HashMap;

use crate::{Platform, preset::Preset};

/// The default opener rules that survive on a system described by `os` and
/// `unix`, in the order the preset lists them.
///
/// This is the real preset, loaded the way `Preset::yazi()` loads it: without
/// the platform filter, which is applied afterwards. Filtering here for a
/// described system instead of the host is what lets the iOS and Android rules
/// be checked from a Linux build.
fn surviving(os: &str, unix: bool) -> HashMap<String, Vec<(Platform, String)>> {
	static INIT: OnceLock<()> = OnceLock::new();
	INIT.get_or_init(|| {
		yazi_shared::init_tests();
		yazi_shim::init().unwrap();
	});

	let yazi = Preset::yazi().unwrap();
	let mut out = HashMap::new();

	for (name, rules) in yazi.opener.load_full().iter() {
		let kept = rules
			.load()
			.iter()
			.filter(|r| r.r#for.matches_on(os, unix))
			.map(|r| (r.r#for, r.run.to_string()))
			.collect();

		out.insert(name.clone(), kept);
	}

	out
}

/// An iOS build reaches the device's own application handoff, and reaches it
/// before the generic Unix commands that would otherwise answer instead.
#[test]
fn test_ios_rules_come_first() {
	let openers = surviving("ios", true);

	for (name, run) in [("open", "ya open %s1"), ("play", "ya open %s1"), ("reveal", "ya open %d1")] {
		let kept = &openers[name];
		let (platform, first) = kept.first().expect("a rule for the platform");

		assert_eq!(*platform, Platform::Ios, "{name}: {kept:?}");
		assert_eq!(first, run, "{name}: {kept:?}");
	}

	// `open` has no Unix rule on any platform, so on iOS it now has exactly
	// the one rule that can hand a document to an application, and nothing
	// unrelated is reachable through it.
	assert_eq!(openers["open"].len(), 1);

	// `play` and `reveal` still offer their Unix metadata commands, just no
	// longer ahead of the handoff.
	for name in ["play", "reveal"] {
		let kept = &openers[name];
		assert!(kept.iter().any(|(p, _)| *p == Platform::Unix), "{name}: {kept:?}");
	}
}

/// iOS is a Unix system, so the editor rule that every other Unix target uses
/// keeps applying, and a GUI handoff is not needed to edit a file on a device
/// with a terminal.
#[test]
fn test_ios_keeps_the_unix_editor() {
	let edit = surviving("ios", true);
	assert_eq!(edit["edit"], [(Platform::Unix, "${EDITOR:-vi} %s".to_owned())]);
}

/// Every other platform keeps the opener it had, and none of them can see the
/// iOS rule.
#[test]
fn test_other_platforms_are_preserved() {
	for (os, unix, platform, run) in [
		("linux", true, Platform::Linux, "xdg-open %s1"),
		("macos", true, Platform::Macos, "open %s"),
		("windows", false, Platform::Windows, r#"start "" %s1"#),
		("android", true, Platform::Android, "termux-open %s1"),
	] {
		let openers = surviving(os, unix);
		let kept = &openers["open"];

		assert_eq!(kept.len(), 1, "{os}: {kept:?}");
		assert_eq!(kept[0], (platform, run.to_owned()), "{os}: {kept:?}");
	}

	// A Unix system that is none of the four has no `open` rule at all, and
	// never the iOS one.
	assert!(surviving("freebsd", true)["open"].is_empty());
}
