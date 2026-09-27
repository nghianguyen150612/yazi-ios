use std::{env, fmt::Write, path::Path, process::Command};

use anyhow::Result;
use yazi_adapter::drivers::{Drivers, kgp_shm_permitted};
use yazi_config::{YAZI, build_flavor};
use yazi_emulator::{Brand, Emulator, Mux};
use yazi_fs::Xdg;
use yazi_shared::timestamp_us;
use yazi_term::TERM;

use crate::{capability::CapabilityReport, env::Env};

impl Env {
	pub(crate) async fn print() -> Result<String> {
		let mut s = String::new();
		let emulator = Emulator::probe().await?;
		let theme = build_flavor(emulator.light().unwrap_or_default())?;
		let capabilities = CapabilityReport::collect();

		writeln!(s, "Yazi\n{}", Self::yazi_version())?;
		writeln!(s, "    Backtrace: {:?}", env::var_os("RUST_BACKTRACE"))?;

		writeln!(s, "\nYa\n{}", yazi_version::version_full())?;

		writeln!(s, "Config")?;
		writeln!(s, "    Init             : {}", Self::config_state("init.lua"))?;
		writeln!(s, "    Yazi             : {}", Self::config_state("yazi.toml"))?;
		writeln!(s, "    Keymap           : {}", Self::config_state("keymap.toml"))?;
		writeln!(s, "    Theme            : {}", Self::config_state("theme.toml"))?;
		writeln!(s, "    VFS              : {}", Self::config_state("vfs.toml"))?;
		writeln!(s, "    Package          : {}", Self::config_state("package.toml"))?;
		writeln!(s, "    Dark/light flavor: {:?} / {:?}", theme.flavor.dark, theme.flavor.light)?;

		writeln!(s, "\nEmulator")?;
		writeln!(s, "    TERM                : {:?}", env::var_os("TERM"))?;
		writeln!(s, "    TERM_PROGRAM        : {:?}", env::var_os("TERM_PROGRAM"))?;
		writeln!(s, "    TERM_PROGRAM_VERSION: {:?}", env::var_os("TERM_PROGRAM_VERSION"))?;
		writeln!(s, "    Brand.from_env      : {:?}", Brand::from_env())?;
		writeln!(s, "    Emulator.probe      : {emulator:?}")?;

		writeln!(s, "\nAdapter")?;
		writeln!(s, "    Drivers.matches   : {:?}", Drivers::matches(&emulator))?;
		writeln!(s, "    Drivers.candidates: {:?}", Drivers::candidates(&emulator))?;
		writeln!(
			s,
			"    Drivers.helpers   : {:?}",
			Drivers::candidates(&emulator)
				.iter()
				.map(|d| match d.helper() {
					Some(h) => format!("{d} (helper: {h})"),
					None => d.to_string(),
				})
				.collect::<Vec<_>>(),
		)?;
		// NOTE: this diagnostic must stay a pure function of the local probe
		// above. The `Command::Env` path never calls `yazi_emulator::init()`
		// (only `yazi-fm` does), so nothing here may dereference the global
		// `EMULATOR` RoCell; in debug builds that asserts, in release it is
		// uninitialized memory.
		let in_ssh = yazi_shared::in_ssh_connection();
		writeln!(
			s,
			"    Kgp SHM           : reported={}, ssh={}, permitted={}",
			emulator.kgp_shm.get(),
			in_ssh,
			kgp_shm_permitted(emulator.kgp_shm.get(), in_ssh),
		)?;
		writeln!(s, "    chafa available   : {}", yazi_adapter::drivers::chafa_available())?;
		writeln!(s, "    TERM.dimension : {:?}", TERM.dimension())?;

		writeln!(s, "\nDesktop")?;
		writeln!(s, "    XDG_SESSION_TYPE           : {:?}", env::var_os("XDG_SESSION_TYPE"))?;
		writeln!(s, "    WAYLAND_DISPLAY            : {:?}", env::var_os("WAYLAND_DISPLAY"))?;
		writeln!(s, "    DISPLAY                    : {:?}", env::var_os("DISPLAY"))?;
		writeln!(s, "    NIRI_SOCKET                : {:?}", env::var_os("NIRI_SOCKET"))?;
		writeln!(s, "    SWAYSOCK                   : {:?}", env::var_os("SWAYSOCK"))?;
		#[rustfmt::skip]
		writeln!(s, "    HYPRLAND_INSTANCE_SIGNATURE: {:?}", env::var_os("HYPRLAND_INSTANCE_SIGNATURE"))?;
		writeln!(s, "    WAYFIRE_SOCKET             : {:?}", env::var_os("WAYFIRE_SOCKET"))?;

		writeln!(s, "\nSSH")?;
		writeln!(s, "    shared.in_ssh_connection: {}", yazi_shared::in_ssh_connection())?;

		writeln!(s, "\nWSL")?;
		writeln!(s, "    WSL: {:?}", yazi_shared::in_wsl())?;

		writeln!(s, "\nVariables")?;
		writeln!(s, "    SHELL              : {:?}", env::var_os("SHELL"))?;
		writeln!(s, "    EDITOR             : {:?}", env::var_os("EDITOR"))?;
		writeln!(s, "    VISUAL             : {:?}", env::var_os("VISUAL"))?;
		writeln!(s, "    YAZI_FILE_ONE      : {:?}", env::var_os("YAZI_FILE_ONE"))?;
		writeln!(s, "    YAZI_CONFIG_HOME   : {:?}", env::var_os("YAZI_CONFIG_HOME"))?;
		writeln!(s, "    YAZI_ZOXIDE_OPTS   : {:?}", env::var_os("YAZI_ZOXIDE_OPTS"))?;
		writeln!(s, "    SSH_AUTH_SOCK      : {:?}", env::var_os("SSH_AUTH_SOCK"))?;
		writeln!(s, "    FZF_DEFAULT_OPTS   : {:?}", env::var_os("FZF_DEFAULT_OPTS"))?;
		writeln!(s, "    FZF_DEFAULT_COMMAND: {:?}", env::var_os("FZF_DEFAULT_COMMAND"))?;

		writeln!(s, "\nText Opener")?;
		#[rustfmt::skip]
		writeln!(
			s, "    default     : {:?}",
			YAZI.open.match_dummy(Path::new("f75a.txt"), "text/plain").and_then(|r| YAZI.opener.first(&r))
		)?;
		#[rustfmt::skip]
		writeln!(
			s, "    block-create: {:?}",
			YAZI.open.match_dummy(Path::new("bulk-create.txt"), "text/plain").and_then(|r| YAZI.opener.block(&r))
		)?;
		#[rustfmt::skip]
		writeln!(
			s, "    block-rename: {:?}",
			YAZI.open.match_dummy(Path::new("bulk-rename.txt"), "text/plain").and_then(|r| YAZI.opener.block(&r))
		)?;

		writeln!(s, "\nMultiplexers")?;
		writeln!(s, "    tmux version       : {}", capabilities.value("tmux"))?;
		writeln!(s, "    tmux build flags   : enable-sixel={}", Mux::tmux_sixel_flag())?;
		writeln!(s, "    ZELLIJ_SESSION_NAME: {:?}", env::var_os("ZELLIJ_SESSION_NAME"))?;
		writeln!(s, "    Zellij version     : {}", capabilities.value("zellij"))?;

		writeln!(s, "\nDependencies")?;
		for line in capabilities.env_lines() {
			writeln!(s, "    {line}")?;
		}

		writeln!(s, "\nClipboard")?;
		writeln!(s, "    wl-copy       : {}", capabilities.value("wl-copy"))?;
		writeln!(s, "    wl-paste      : {}", capabilities.value("wl-paste"))?;
		writeln!(s, "    xclip         : {}", capabilities.value("xclip"))?;
		writeln!(s, "    xsel          : {}", capabilities.value("xsel"))?;

		writeln!(s, "\nRoutine")?;
		writeln!(s, "    `file -bL --mime-type`: {}", Self::file1_output())?;

		Ok(s)
	}

	fn yazi_version() -> String {
		match Command::new("yazi").arg("--version").output() {
			Ok(out) if out.status.success() => {
				let s = if out.stdout.is_empty() { out.stderr } else { out.stdout };
				let s = String::from_utf8_lossy(&s);
				let s = s.trim();
				s.strip_prefix("Yazi\n").unwrap_or(s).to_owned()
			}
			Ok(out) => format!("{:?}, {:?}", out.status, String::from_utf8_lossy(&out.stderr)),
			Err(e) => format!("{e}"),
		}
	}

	fn config_state(name: &str) -> String {
		let p = Xdg::config_dir().join(name);
		match std::fs::read_to_string(&p) {
			Ok(s) if s.is_empty() => format!("{} (empty)", p.display()),
			Ok(s) if s.trim().is_empty() => format!("{} (whitespaces)", p.display()),
			Ok(s) => format!("{} ({} chars)", p.display(), s.chars().count()),
			Err(e) => format!("{} ({e})", p.display()),
		}
	}

	fn file1_output() -> String {
		use std::io::Write;

		let p = env::temp_dir().join(format!(".yazi-debug-{}.tmp", timestamp_us()));
		std::fs::File::create_new(&p).map(|mut f| f.write_all(b"Hello, World!")).ok();

		let program = env::var_os("YAZI_FILE_ONE").unwrap_or("file".into());
		match Command::new(program).args(["-bL", "--mime-type"]).arg(&p).output() {
			Ok(out) => {
				String::from_utf8_lossy(&out.stdout).trim().lines().next().unwrap_or_default().to_owned()
			}
			Err(e) => format!("{e}"),
		}
	}
}
