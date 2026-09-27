use std::path::PathBuf;

use anyhow::Result;
use ratatui_core::layout::Rect;
use strum::{Display, IntoStaticStr};

use crate::drivers::{Chafa, Iip, Kgp, KgpOld, Sixel, Ueberzug};

#[derive(Clone, Copy, Debug, Display, Eq, IntoStaticStr, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum Driver {
	Kgp,
	KgpOld,
	Iip,
	Sixel,

	// Supported by Überzug++
	X11,
	Wayland,
	Chafa,
}

impl Driver {
	pub(crate) async fn image_show<P>(self, path: P, max: Rect) -> Result<Rect>
	where
		P: Into<PathBuf>,
	{
		if max.is_empty() {
			return Ok(Rect::default());
		}

		let path = path.into();
		match self {
			Self::Kgp => Kgp::image_show(path, max).await,
			Self::KgpOld => KgpOld::image_show(path, max).await,
			Self::Iip => Iip::image_show(path, max).await,
			Self::Sixel => Sixel::image_show(path, max).await,
			Self::X11 | Self::Wayland => Ueberzug::image_show(path, max).await,
			Self::Chafa => Chafa::image_show(path, max).await,
		}
	}

	pub(crate) fn image_erase(self, area: Rect) -> Result<()> {
		match self {
			Self::Kgp => Kgp::image_erase(area),
			Self::KgpOld => KgpOld::image_erase(area),
			Self::Iip => Iip::image_erase(area),
			Self::Sixel => Sixel::image_erase(area),
			Self::X11 | Self::Wayland => Ueberzug::image_erase(area),
			Self::Chafa => Chafa::image_erase(area),
		}
	}

	pub(crate) fn start(self) { Ueberzug::start(self); }

	pub(crate) fn needs_ueberzug(self) -> bool { matches!(self, Self::X11 | Self::Wayland) }

	// Terminal-native protocols encode in-process and write escape sequences
	// directly; they never spawn an external renderer.
	pub fn is_terminal(self) -> bool {
		matches!(self, Self::Kgp | Self::KgpOld | Self::Iip | Self::Sixel)
	}

	// External helper the driver shells out to, if any. Chafa renders through
	// the `chafa` binary directly; X11/Wayland draw through the `ueberzugpp`
	// daemon. Terminal-native drivers need no helper.
	pub fn helper(self) -> Option<&'static str> {
		match self {
			Self::Chafa => Some("chafa"),
			Self::X11 | Self::Wayland => Some("ueberzugpp"),
			_ => None,
		}
	}

	// Whether failing over to the next candidate after this driver errors is
	// safe. Helper-backed drivers fail observably before committing terminal
	// state (a missing binary fails at spawn, an unstarted daemon bails
	// before any escape sequence is written), so falling through cannot
	// duplicate partial output. Terminal-native drivers encode and then write
	// in one step; a failure there may follow partial terminal output, and a
	// terminal that silently ignores an unsupported sequence reports success
	// at the OS write level, so there is no acknowledgement to retry on.
	// Those errors surface immediately instead.
	pub(crate) fn failover_safe(self) -> bool { !self.is_terminal() }
}

// Whether the Kitty Graphics shared-memory transport may be used.
// `reported` is the terminal's answer to the active `RequestKgpShm` probe.
// Shared memory is host-local by protocol design: the terminal opens the
// object the client created with `shm_open`, so both sides must share one
// machine (Kitty graphics protocol, "The transmission medium": remote
// clients "unable to use the filesystem/shared memory ... must send the
// pixel data directly"). Over SSH the terminal runs on another machine and
// can never open an object created on this one, so KGP stays available but
// is forced onto its base64/direct transport.
pub fn kgp_shm_permitted(reported: bool, in_ssh: bool) -> bool { reported && !in_ssh }

pub fn kgp_shm_allowed() -> bool {
	use yazi_emulator::EMULATOR;

	kgp_shm_permitted(EMULATOR.kgp_shm.get(), yazi_shared::in_ssh_connection())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ueberzug_classification() {
		// Only the compositor drivers draw through the ueberzugpp daemon.
		// Chafa spawns `chafa` directly and must never start ueberzugpp.
		assert!(Driver::X11.needs_ueberzug());
		assert!(Driver::Wayland.needs_ueberzug());
		assert!(!Driver::Chafa.needs_ueberzug());
		assert!(!Driver::Kgp.needs_ueberzug());
		assert!(!Driver::KgpOld.needs_ueberzug());
		assert!(!Driver::Iip.needs_ueberzug());
		assert!(!Driver::Sixel.needs_ueberzug());
	}

	#[test]
	fn helper_boundary() {
		assert_eq!(Driver::Chafa.helper(), Some("chafa"));
		assert_eq!(Driver::X11.helper(), Some("ueberzugpp"));
		assert_eq!(Driver::Wayland.helper(), Some("ueberzugpp"));
		assert_eq!(Driver::Kgp.helper(), None);
		assert_eq!(Driver::KgpOld.helper(), None);
		assert_eq!(Driver::Iip.helper(), None);
		assert_eq!(Driver::Sixel.helper(), None);
	}

	#[test]
	fn failover_policy() {
		// Helper failures precede any terminal commit and may fall through;
		// terminal-protocol failures surface immediately.
		assert!(Driver::Chafa.failover_safe());
		assert!(Driver::X11.failover_safe());
		assert!(Driver::Wayland.failover_safe());
		assert!(!Driver::Kgp.failover_safe());
		assert!(!Driver::KgpOld.failover_safe());
		assert!(!Driver::Iip.failover_safe());
		assert!(!Driver::Sixel.failover_safe());
	}

	#[test]
	fn shm_transport_policy() {
		// Local terminal reporting SHM support may use it.
		assert!(kgp_shm_permitted(true, false));
		// No SHM support reported means base64, with or without SSH.
		assert!(!kgp_shm_permitted(false, false));
		assert!(!kgp_shm_permitted(false, true));
		// Over SSH the terminal is remote and cannot open this host's SHM
		// object, so KGP must use base64 even when SHM was reported.
		assert!(!kgp_shm_permitted(true, true));
	}
}
