//! The clipboard of a desktop or Android Unix host is whatever helper command
//! its session provides. iOS is excluded: an iOS rootfs ships none of them,
//! so probing would only spawn failing processes.

use super::{Clipboard, lookup};
use tokio::process::Command;

/// Clipboard readers to try, most specific first.
const GETTERS: &[(&str, &[&str])] = &[
	("pbpaste", &[]),
	("termux-clipboard-get", &[]),
	("wl-paste", &["-n"]),
	("xclip", &["-o", "-selection", "clipboard"]),
	("xsel", &["-ob"]),
];

/// Clipboard writers to try, most specific first.
const SETTERS: &[(&str, &[&str])] = &[
	("pbcopy", &[]),
	("termux-clipboard-set", &[]),
	("wl-copy", &[]),
	("xclip", &["-selection", "clipboard"]),
	("xsel", &["-ib"]),
];

impl Clipboard {
	pub async fn get(&self) -> Vec<u8> {
		lookup::read(
			async {
				for &(bin, args) in GETTERS {
					let Ok(output) = Command::new(bin).args(args).kill_on_drop(true).output().await else {
						continue;
					};
					if output.status.success() {
						return Some(output.stdout);
					}
				}
				None
			},
			self.mirrored(),
		)
		.await
	}

	pub async fn set(&self, s: impl AsRef<[u8]>) {
		use std::process::Stdio;

		use tokio::io::AsyncWriteExt;
		use yazi_macro::writef;
		use yazi_tty::{TTY, sequence::SetClipboard};

		self.mirror(s.as_ref());
		writef!(TTY.writer(), "{}", SetClipboard(s.as_ref())).ok();

		for &(bin, args) in SETTERS {
			let cmd = Command::new(bin)
				.args(args)
				.stdin(Stdio::piped())
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.kill_on_drop(true)
				.spawn();

			let Ok(mut child) = cmd else { continue };

			let mut stdin = child.stdin.take().unwrap();
			if stdin.write_all(s.as_ref()).await.is_err() {
				continue;
			}
			drop(stdin);

			if child.wait().await.map(|s| s.success()).unwrap_or_default() {
				break;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{GETTERS, SETTERS};

	/// The probe list is a contract, not a wish list: every entry spawns a
	/// process, so a new one has to earn its place. iOS must never appear here
	/// — it has its own native backend in `super::ios`.
	#[test]
	fn probes_only_helpers_a_unix_session_can_have() {
		assert_eq!(
			GETTERS.iter().map(|&(bin, _)| bin).collect::<Vec<_>>(),
			["pbpaste", "termux-clipboard-get", "wl-paste", "xclip", "xsel"]
		);
		assert_eq!(
			SETTERS.iter().map(|&(bin, _)| bin).collect::<Vec<_>>(),
			["pbcopy", "termux-clipboard-set", "wl-copy", "xclip", "xsel"]
		);
	}
}
