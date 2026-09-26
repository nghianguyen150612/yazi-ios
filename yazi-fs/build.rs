fn main() {
	cfg_aliases::cfg_aliases! {
		trash_unsupported: {
			target_os = "android"
		},
		trash_unix: {
			all(unix, not(trash_unsupported))
		},
		trash_freedesktop: {
			all(unix, not(target_os = "macos"), not(trash_unsupported))
		},
	}
}
