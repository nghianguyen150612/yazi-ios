use std::{env, ops::{Deref, DerefMut}};

use yazi_emulator::{Brand, Emulator};
use yazi_macro::warn;
use yazi_shared::env_exists;

use crate::drivers::{Driver as D, Ueberzug};

pub struct Drivers(Vec<D>);

impl Deref for Drivers {
	type Target = Vec<D>;

	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Drivers {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<&Emulator> for Drivers {
	fn from(value: &Emulator) -> Self {
		match value.brand.get() {
			Brand::Unknown => Self(match (value.kgp.get(), value.sixel.get()) {
				(true, true) => vec![D::Sixel, D::KgpOld],
				(true, false) => vec![D::KgpOld],
				(false, true) => vec![D::Sixel],
				(false, false) => vec![],
			}),
			Brand::Zellij => Self(match (value.kgp.get(), value.sixel.get()) {
				(true, true) => vec![D::Sixel, D::KgpOld],
				(true, false) => vec![D::KgpOld],
				(false, true) => vec![D::Sixel],
				(false, false) => vec![],
			}),
			brand => brand.into(),
		}
	}
}

impl From<Brand> for Drivers {
	fn from(value: Brand) -> Self {
		use Brand as B;

		Self(match value {
			B::Unknown => vec![],
			B::Kitty => vec![D::Kgp],
			B::Konsole => vec![D::KgpOld],
			B::Iterm2 => vec![D::Iip, D::Sixel],
			B::WezTerm => vec![D::Iip, D::Sixel],
			B::Foot => vec![D::Sixel],
			B::Ghostty => vec![D::Kgp],
			B::Microsoft => vec![D::Sixel],
			B::Warp => vec![D::Iip, D::KgpOld],
			B::Rio => vec![D::Kgp],
			B::BlackBox => vec![D::Sixel],
			B::VSCode => vec![D::Iip, D::Sixel],
			B::Tabby => vec![D::Iip, D::Sixel],
			B::Hyper => vec![D::Iip, D::Sixel],
			B::Mintty => vec![D::Iip],
			B::Tmux => vec![],
			B::Zellij => vec![],
			B::VTerm => vec![],
			B::Apple => vec![],
			B::Urxvt => vec![],
			B::Bobcat => vec![D::Iip, D::Sixel],
		})
	}
}

// Terminal capability evidence, injected so selection stays a pure function
// of probe results rather than of global emulator state.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProbeState {
	pub brand: Brand,
	pub kgp:   bool,
	pub sixel: bool,
	// `None` when not inside tmux, `Some` carrying the mux sixel flag otherwise.
	pub mux:   Option<bool>,
}

impl From<&Emulator> for ProbeState {
	fn from(value: &Emulator) -> Self {
		Self {
			brand: value.brand.get(),
			kgp:   value.kgp.get(),
			sixel: value.sixel.get(),
			mux:   value.mux.get().map(|mux| mux.sixel),
		}
	}
}

// Desktop compositor evidence, injected so tests never touch process env.
#[derive(Clone, Debug, Default)]
pub struct DesktopEnv {
	pub xdg_session_type:     String,
	pub wayland_display:      bool,
	pub display:              String,
	pub supported_compositor: bool,
}

impl DesktopEnv {
	pub fn live() -> Self {
		Self {
			xdg_session_type: env::var("XDG_SESSION_TYPE").unwrap_or_default(),
			wayland_display: env_exists("WAYLAND_DISPLAY"),
			display: env::var("DISPLAY").unwrap_or_default(),
			supported_compositor: Ueberzug::supported_compositor(),
		}
	}

	pub fn empty() -> Self { Self::default() }
}

impl Drivers {
	// Ordered native candidates for a probe state. This preserves the
	// established compatibility facts:
	// - modern KGP (`Kgp`, unicode placeholders) and legacy placement
	//   (`KgpOld`) are not interchangeable: known Kitty-protocol brands map
	//   to `Kgp`, while an unknown terminal whose only evidence is a
	//   successful KGP query gets legacy `KgpOld` placement.
	// - Sixel is selected when DA1 positively reports attribute 4.
	// - IIP has no active protocol query and stays identity-driven, ordered
	//   ahead of Sixel for the known IIP brands.
	// - inside tmux, legacy placement is dropped unless the mux sixel path
	//   positively selects Sixel.
	fn native(probe: &ProbeState) -> Vec<D> {
		let mut drivers: Vec<D> = match probe.brand {
			Brand::Unknown | Brand::Zellij => match (probe.kgp, probe.sixel) {
				(true, true) => vec![D::Sixel, D::KgpOld],
				(true, false) => vec![D::KgpOld],
				(false, true) => vec![D::Sixel],
				(false, false) => vec![],
			},
			brand => Self::from(brand).0,
		};
		if probe.sixel && probe.mux == Some(true) {
			return vec![D::Sixel];
		} else if probe.mux.is_some() {
			drivers.retain(|p| *p != D::KgpOld);
		}
		drivers
	}

	// Ordered candidates for a probe state. When native protocols are
	// evidenced this is exactly the native list in the verified brand order
	// (`native()` above); the platform fallback is used only when there is
	// no native candidate. That is deliberate: terminal-native runtime
	// errors surface rather than being retried, because a failure there may
	// follow partial escape output and a silently ignored sequence has no
	// acknowledgement to retry on. Only helper-backed failures (missing
	// `chafa`, unstarted daemon: observably pre-commit) fall through, and
	// those live at the tail. Never empty: with no native candidate the tail
	// is at least Chafa, which itself degrades to the Lua metadata preview
	// when its helper is absent.
	pub fn candidates_for(probe: &ProbeState, env: &DesktopEnv, ios: bool) -> Vec<D> {
		let native = Self::native(probe);
		if !native.is_empty() {
			return native;
		}
		if ios {
			// A jailbroken iOS local terminal has no desktop compositor;
			// weak compositor env evidence (possibly forwarded over SSH)
			// must not turn it into an X11/Wayland client.
			return vec![D::Chafa];
		}
		vec![Self::desktop_fallback(env)]
	}

	fn desktop_fallback(env: &DesktopEnv) -> D {
		match env.xdg_session_type.as_str() {
			"x11" => return D::X11,
			"wayland" if env.supported_compositor => return D::Wayland,
			"wayland" if !env.supported_compositor => return D::Chafa,
			_ => warn!("[Drivers] Could not identify XDG_SESSION_TYPE"),
		}
		if env.wayland_display {
			return if env.supported_compositor { D::Wayland } else { D::Chafa };
		}
		match env.display.as_str() {
			s if !s.is_empty() && !s.contains("/org.xquartz") => return D::X11,
			_ => {}
		}

		warn!("[Drivers] Falling back to chafa");
		D::Chafa
	}

	pub fn candidates(emu: &Emulator) -> Vec<D> {
		Self::candidates_for(&emu.into(), &DesktopEnv::live(), cfg!(target_os = "ios"))
	}

	pub fn matches(emu: &Emulator) -> D {
		// The chain is never empty, so a selected renderer always exists;
		// failover past it lives in `Adapter`, not here.
		Self::candidates(emu).into_iter().next().unwrap_or(D::Chafa)
	}
}

// Best-effort `chafa` availability probe for diagnostics only. Rendering
// itself just attempts the spawn and falls through on failure, so this is
// never a startup gate.
pub fn chafa_available() -> bool {
	const EXE: &str = if cfg!(windows) { "chafa.exe" } else { "chafa" };

	env::split_paths(&env::var_os("PATH").unwrap_or_default()).any(|dir| {
		if std::fs::symlink_metadata(dir.join(EXE)).is_ok_and(|m| m.is_file()) {
			return true;
		}
		#[cfg(windows)]
		{
			std::fs::symlink_metadata(dir.join("chafa.bat")).is_ok_and(|m| m.is_file())
		}
		#[cfg(not(windows))]
		{
			false
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn probe(brand: Brand, kgp: bool, sixel: bool, mux: Option<bool>) -> ProbeState {
		ProbeState { brand, kgp, sixel, mux }
	}

	fn desktop() -> DesktopEnv { DesktopEnv::empty() }

	#[test]
	fn known_kitty_stays_kgp() {
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Kitty, true, false, None), &desktop(), false),
			vec![D::Kgp]
		);
	}

	#[test]
	fn unknown_with_kgp_gets_legacy_placement() {
		// An unknown terminal with only a successful KGP query is not known
		// to support modern unicode-placeholder KGP, so it gets legacy
		// placement rather than `Kgp`.
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, true, false, None), &desktop(), false),
			vec![D::KgpOld]
		);
	}

	#[test]
	fn unknown_with_sixel_only() {
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, false, true, None), &desktop(), false),
			vec![D::Sixel]
		);
	}

	#[test]
	fn kgp_and_sixel_prefers_sixel_for_unknown() {
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, true, true, None), &desktop(), false),
			vec![D::Sixel, D::KgpOld]
		);
	}

	#[test]
	fn iterm2_and_wezterm_lead_with_iip() {
		// IIP is identity-driven (no active query exists) and ordered ahead
		// of Sixel for the known IIP brands.
		for brand in [Brand::Iterm2, Brand::WezTerm] {
			assert_eq!(
				Drivers::candidates_for(&probe(brand, false, true, None), &desktop(), false),
				vec![D::Iip, D::Sixel]
			);
		}
	}

	#[test]
	fn tmux_with_sixel_mux_selects_sixel() {
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, true, true, Some(true)), &desktop(), false),
			vec![D::Sixel]
		);
	}

	#[test]
	fn tmux_without_sixel_mux_drops_legacy_kgp() {
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, true, true, Some(false)), &desktop(), false),
			vec![D::Sixel]
		);
		// Legacy placement is dropped inside tmux; with nothing native
		// left, the chain ends in the platform fallback.
		assert_eq!(
			Drivers::candidates_for(&probe(Brand::Unknown, true, false, Some(false)), &desktop(), false),
			vec![D::Chafa]
		);
	}

	#[test]
	fn no_native_graphics_falls_back() {
		let p = probe(Brand::Unknown, false, false, None);
		assert_eq!(Drivers::candidates_for(&p, &desktop(), false), vec![D::Chafa]);
		// iOS never invents a desktop compositor from env evidence.
		assert_eq!(Drivers::candidates_for(&p, &desktop(), true), vec![D::Chafa]);
	}

	#[test]
	fn ios_ignores_compositor_env() {
		let p = probe(Brand::Unknown, false, false, None);
		let mut env = DesktopEnv::empty();
		env.xdg_session_type = "x11".to_owned();
		env.display = ":0".to_owned();
		env.wayland_display = true;
		env.supported_compositor = true;
		assert_eq!(Drivers::candidates_for(&p, &env, true), vec![D::Chafa]);
	}

	#[test]
	fn desktop_policy_preserved() {
		let p = probe(Brand::Unknown, false, false, None);
		let mut env = DesktopEnv::empty();
		env.xdg_session_type = "x11".to_owned();
		assert_eq!(Drivers::candidates_for(&p, &env, false), vec![D::X11]);

		let mut env = DesktopEnv::empty();
		env.xdg_session_type = "wayland".to_owned();
		env.supported_compositor = true;
		assert_eq!(Drivers::candidates_for(&p, &env, false), vec![D::Wayland]);

		let mut env = DesktopEnv::empty();
		env.xdg_session_type = "wayland".to_owned();
		assert_eq!(Drivers::candidates_for(&p, &env, false), vec![D::Chafa]);

		let mut env = DesktopEnv::empty();
		env.display = ":0".to_owned();
		assert_eq!(Drivers::candidates_for(&p, &env, false), vec![D::X11]);
	}

	#[test]
	fn matches_selects_chain_head() {
		let p = probe(Brand::WezTerm, true, true, None);
		let chain = Drivers::candidates_for(&p, &desktop(), false);
		assert_eq!(chain, vec![D::Iip, D::Sixel]);
	}
}
