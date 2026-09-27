#[macro_export]
macro_rules! config_preset {
	($name:literal) => {{
		// iOS debug binaries are copied to the device without the build-machine
		// source tree, so they must embed presets like release does. Desktop
		// debug keeps source-tree loading for developer convenience.
		#[cfg(all(debug_assertions, not(target_os = "ios")))]
		{
			std::borrow::Cow::from(
				std::fs::read_to_string(concat!(
					env!("CARGO_MANIFEST_DIR"),
					"/../yazi-config/preset/",
					$name,
					"-default.toml"
				))
				.expect(concat!("Failed to read 'yazi-config/preset/", $name, "-default.toml'")),
			)
		}
		#[cfg(any(not(debug_assertions), target_os = "ios"))]
		{
			std::borrow::Cow::from(include_str!(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/../yazi-config/preset/",
				$name,
				"-default.toml"
			)))
		}
	}};
}

#[macro_export]
macro_rules! plugin_preset {
	($name:literal) => {{
		// See config_preset!: iOS debug embeds presets so a device binary is
		// self-contained; desktop debug reads from the source tree.
		#[cfg(all(debug_assertions, not(target_os = "ios")))]
		{
			std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../yazi-plugin/preset/", $name, ".lua"))
				.expect(concat!("Failed to read 'yazi-plugin/preset/", $name, ".lua'"))
		}
		#[cfg(any(not(debug_assertions), target_os = "ios"))]
		{
			&include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../yazi-plugin/preset/", $name, ".lua"))
				[..]
		}
	}};
}

#[macro_export]
macro_rules! theme_preset {
	($name:literal) => {{
		// See config_preset!: iOS debug embeds presets so a device binary is
		// self-contained; desktop debug reads from the source tree.
		#[cfg(all(debug_assertions, not(target_os = "ios")))]
		{
			std::borrow::Cow::from(
				std::fs::read_to_string(concat!(
					env!("CARGO_MANIFEST_DIR"),
					"/preset/theme-",
					$name,
					".toml"
				))
				.expect(concat!("Failed to read 'yazi-config/preset/theme-", $name, ".toml'")),
			)
		}
		#[cfg(any(not(debug_assertions), target_os = "ios"))]
		{
			std::borrow::Cow::from(include_str!(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/preset/theme-",
				$name,
				".toml"
			)))
		}
	}};
}
