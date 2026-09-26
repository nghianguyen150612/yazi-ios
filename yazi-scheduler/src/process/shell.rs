use std::{ffi::OsString, process::Stdio};

use anyhow::{Error, Result};
use tokio::{process::{Child, Command}, task};
use yazi_fs::Cwd;
use yazi_macro::impl_data_any;
use yazi_shared::url::{AsUrl, UrlBuf};

/// Resolved through `PATH` by `execvp(3)`, so the environment of whoever
/// launched Yazi decides which shell interprets the command. It is deliberately
/// not pinned to a specific absolute path, since jailbreak layouts differ
/// between rootful and rootless environments.
#[cfg(unix)]
const SHELL: &str = "sh";

#[cfg(unix)]
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ShellOpt {
	pub cwd:    UrlBuf,
	pub cmd:    OsString,
	pub block:  bool,
	pub orphan: bool,
}

impl_data_any!(ShellOpt);

impl ShellOpt {
	#[inline]
	fn stdio(&self) -> Stdio {
		if self.block {
			Stdio::inherit()
		} else if self.orphan {
			Stdio::null()
		} else {
			Stdio::piped()
		}
	}
}

pub(crate) async fn shell(opt: ShellOpt) -> Result<Child> {
	let (cwd, opt) =
		task::spawn_blocking(move || (Cwd::ensure(opt.cwd.as_url()).into_owned(), opt)).await?;

	#[cfg(unix)]
	return Ok(unsafe {
		Command::new(SHELL)
			.stdin(opt.stdio())
			.stdout(opt.stdio())
			.stderr(opt.stdio())
			.arg("-c")
			.arg(opt.cmd)
			.current_dir(&cwd)
			.kill_on_drop(!opt.orphan)
			// `setsid()` detaches non-blocking commands from Yazi's session, so they
			// neither take terminal signals nor die with it. It has to run between
			// `fork` and `exec`, which `pre_exec` provides; std has no stable
			// equivalent, and `posix_spawn` cannot express a new session on Apple.
			.pre_exec(move || {
				if !opt.block && libc::setsid() < 0 {
					return Err(std::io::Error::last_os_error());
				}
				Ok(())
			})
			.spawn()
			.map_err(|e| spawn_error(SHELL, &cwd, e))?
	});

	#[cfg(windows)]
	return Ok(
		Command::new("cmd.exe")
			.stdin(opt.stdio())
			.stdout(opt.stdio())
			.stderr(opt.stdio())
			.env("=", r#""^\n\n""#)
			.raw_arg(r#"/Q /S /D /V:OFF /E:ON /C ""#)
			.raw_arg(opt.cmd)
			.raw_arg(r#"""#)
			.current_dir(cwd)
			.kill_on_drop(!opt.orphan)
			.spawn()?,
	);
}

/// Names the reason a spawn failed instead of surfacing a bare errno.
///
/// The child enters the working directory before it resolves the shell, so a
/// directory that has gone away and a shell that is not installed both report
/// `NotFound`. Distinguishing them keeps the failure actionable, which matters
/// most where `PATH` is minimal or the directory came from a path that is no
/// longer valid.
#[cfg(unix)]
fn spawn_error(shell: &str, cwd: &Path, err: std::io::Error) -> Error {
	use std::io::ErrorKind::{NotFound, PermissionDenied};

	let cause = match err.kind() {
		NotFound if !cwd.is_dir() => {
			format!("the working directory `{}` is not accessible", cwd.display())
		}
		NotFound => format!("`{shell}` was not found in `PATH`"),
		PermissionDenied => "process creation was denied".to_string(),
		_ => return Error::new(err).context(format!("Failed to run `{shell}`")),
	};

	Error::new(err).context(format!("Failed to run `{shell}`: {cause}"))
}

#[cfg(all(test, unix))]
mod tests {
	use std::io::ErrorKind;

	use super::*;

	fn dir() -> std::path::PathBuf {
		let dir = std::env::temp_dir().join("yazi-scheduler-shell-tests");
		std::fs::create_dir_all(&dir).unwrap();
		dir
	}

	fn missing_dir() -> std::path::PathBuf {
		std::env::temp_dir().join("yazi-scheduler-shell-tests-does-not-exist")
	}

	/// A missing shell and an unreachable working directory produce the same
	/// errno from `spawn`, so the diagnostic has to tell them apart.
	#[test]
	fn names_a_missing_working_directory() {
		let err = spawn_error(SHELL, &missing_dir(), ErrorKind::NotFound.into());
		assert!(err.to_string().contains("working directory"), "{err}");
	}

	#[test]
	fn names_a_missing_shell() {
		let err = spawn_error(SHELL, &dir(), ErrorKind::NotFound.into());
		assert!(err.to_string().contains("not found in `PATH`"), "{err}");
	}

	#[test]
	fn names_a_denied_process_creation() {
		let err = spawn_error(SHELL, &dir(), ErrorKind::PermissionDenied.into());
		assert!(err.to_string().contains("denied"), "{err}");
	}

	/// Anything else keeps the original error as the source instead of guessing.
	#[test]
	fn keeps_the_original_cause() {
		let err = spawn_error(SHELL, &dir(), ErrorKind::InvalidInput.into());
		assert!(err.source().is_some(), "{err}");
	}

	#[tokio::test]
	async fn blocking_command_reports_its_exit_status() {
		let mut child = shell(ShellOpt {
			cwd:   dir().into(),
			cmd:   "exit 3".into(),
			block: true,
			orphan: false,
		})
		.await
		.unwrap();

		assert_eq!(child.wait().await.unwrap().code(), Some(3));
	}

	#[tokio::test]
	async fn runs_the_command_in_the_requested_directory() {
		let dir = dir();
		let mut child = shell(ShellOpt {
			cwd:   dir.clone().into(),
			cmd:   "pwd".into(),
			block: false,
			orphan: false,
		})
		.await
		.unwrap();

		let mut out = String::new();
		{
			use tokio::io::AsyncReadExt;
			let mut stdout = child.stdout.take().unwrap();
			stdout.read_to_string(&mut out).await.unwrap();
		}

		// macOS reports `/private/var` for `/var`, so compare canonical paths.
		assert_eq!(
			std::fs::canonicalize(out.trim()).unwrap(),
			std::fs::canonicalize(dir).unwrap()
		);
	}

	/// A non-blocking command is detached from Yazi's session, which must not
	/// surface as a spawn failure.
	#[tokio::test]
	async fn non_blocking_command_detaches() {
		let mut child = shell(ShellOpt {
			cwd:   dir().into(),
			cmd:   "exit 0".into(),
			block: false,
			orphan: false,
		})
		.await
		.unwrap();

		assert!(child.wait().await.unwrap().success());
	}

	/// An orphan keeps running after Yazi drops the handle, so it must not be
	/// killed on drop and must detach without error.
	#[tokio::test]
	async fn orphan_command_detaches() {
		let child = shell(ShellOpt {
			cwd:   dir().into(),
			cmd:   "exit 0".into(),
			block: false,
			orphan: true,
		})
		.await
		.unwrap();

		drop(child);
	}

	#[tokio::test]
	async fn reports_an_unusable_working_directory() {
		let err = shell(ShellOpt {
			cwd:   missing_dir().into(),
			cmd:   "exit 0".into(),
			block: true,
			orphan: false,
		})
		.await
		.unwrap_err();

		assert!(err.to_string().contains("working directory"), "{err}");
	}
}
