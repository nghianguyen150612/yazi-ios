use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
	#[default]
	All,
	Linux,
	Macos,
	Windows,
	Android,
	/// Its own platform, and still a Unix one: `for = "unix"` keeps matching
	/// iOS, so an iOS rule can be placed in front of a generic Unix fallback.
	Ios,
	Unix,
}

impl Platform {
	/// Whether a rule written for this platform applies to the running system.
	pub(crate) fn matches(self) -> bool {
		self.matches_on(std::env::consts::OS, cfg!(unix))
	}

	/// The same question, asked about a described system instead of this one.
	///
	/// Being a Unix system is a property of the target rather than a name, which
	/// is why iOS gets an entry of its own and still answers to `unix`: it is
	/// not macOS and not Android, but it *is* Unix, so an iOS-specific rule can
	/// sit in front of a generic Unix fallback and still be reachable.
	pub(crate) fn matches_on(self, os: &str, unix: bool) -> bool {
		match self {
			Self::All => true,
			Self::Linux => os == "linux",
			Self::Macos => os == "macos",
			Self::Ios => os == "ios",
			Self::Windows => os == "windows",
			Self::Android => os == "android",
			Self::Unix => unix,
		}
	}
}

// --- Tests
#[cfg(test)]
mod tests {
	use serde::Deserialize;

	use super::Platform;

	/// Deserializes the spelling exactly as a rule table carries it.
	fn parse(s: &str) -> Result<Platform, toml::de::Error> {
		#[derive(Deserialize)]
		struct Rule {
			#[serde(rename = "for")]
			platform: Platform,
		}

		toml::from_str::<Rule>(&format!("for = {s:?}")).map(|r| r.platform)
	}

	/// The spelling in `for = "..."` is kebab-case, and an unknown platform is
	/// a configuration error rather than a rule that quietly never matches.
	#[test]
	fn test_parse() {
		for (s, expected) in [
			("all", Platform::All),
			("linux", Platform::Linux),
			("macos", Platform::Macos),
			("windows", Platform::Windows),
			("android", Platform::Android),
			("ios", Platform::Ios),
			("unix", Platform::Unix),
		] {
			assert_eq!(parse(s).unwrap(), expected, "{s}");
		}

		assert!(parse("iphoneos").is_err());
		assert!(parse("iOS").is_err());
		assert!(parse("darwin").is_err());
	}

	/// iOS is its own platform, so an iOS rule reaches an iOS device and
	/// nothing else.
	#[test]
	fn test_ios_only() {
		for (os, unix) in [("ios", true), ("macos", true), ("android", true), ("linux", true)] {
			assert_eq!(Platform::Ios.matches_on(os, unix), os == "ios", "{os}");
		}
		assert!(!Platform::Ios.matches_on("windows", false));
	}

	/// iOS is still Unix, so the generic Unix rules — including the editor —
	/// keep answering there, and an iOS rule can be placed ahead of them.
	#[test]
	fn test_unix_matches_ios() {
		for os in ["linux", "macos", "ios", "android", "freebsd"] {
			assert!(Platform::Unix.matches_on(os, true), "{os}");
		}

		assert!(!Platform::Unix.matches_on("windows", false));
	}

	/// Every other platform keeps matching exactly what it did before, so
	/// adding iOS cannot change a Linux, macOS, Windows, or Android build.
	#[test]
	fn test_other_platforms() {
		assert!(Platform::All.matches_on("windows", false));

		assert!(Platform::Linux.matches_on("linux", true));
		assert!(!Platform::Linux.matches_on("ios", true));

		assert!(Platform::Macos.matches_on("macos", true));
		assert!(!Platform::Macos.matches_on("ios", true));

		assert!(Platform::Windows.matches_on("windows", false));
		assert!(!Platform::Windows.matches_on("ios", true));

		assert!(Platform::Android.matches_on("android", true));
		assert!(!Platform::Android.matches_on("ios", true));
	}

	/// The running build answers through the same table, so `matches()` and
	/// `matches_on()` cannot drift apart.
	#[test]
	fn test_the_host() {
		let all = [
			Platform::All,
			Platform::Linux,
			Platform::Macos,
			Platform::Windows,
			Platform::Android,
			Platform::Ios,
			Platform::Unix,
		];

		for p in all {
			assert_eq!(p.matches(), p.matches_on(std::env::consts::OS, cfg!(unix)), "{p:?}");
		}
	}
}
