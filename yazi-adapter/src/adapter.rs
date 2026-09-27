use std::{fmt::{self, Debug}, path::PathBuf};

use anyhow::Result;
use ratatui_core::layout::Rect;
use yazi_emulator::EMULATOR;
use yazi_shim::cell::SyncCell;
use yazi_widgets::clear::ClearInventory;

use crate::{ADAPTOR, drivers::{Driver, Drivers}};

#[derive(Default)]
pub struct Adapter {
	selected:      SyncCell<Option<Driver>>,
	shown:         SyncCell<Option<Rect>>,
	pub collision: SyncCell<bool>,
}

impl Debug for Adapter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.selected.get() {
			Some(driver) => driver.fmt(f),
			None => f.write_str("Pending"),
		}
	}
}

impl Adapter {
	pub async fn image_show<P>(&self, path: P, max: Rect) -> Result<Rect>
	where
		P: Into<PathBuf>,
	{
		let probe = &EMULATOR.probe;
		probe.wait(probe.id.get()).await;

		let path: PathBuf = path.into();
		let mut err = None;
		for driver in self.order() {
			driver.start();
			match driver.image_show(path.clone(), max).await {
				Ok(area) => {
					self.selected.set(Some(driver));
					return Ok(area);
				}
				// Never discard `shown` tracking here. A driver can fail
				// before `ADAPTOR.image_hide()` (decode/spawn error), in
				// which case the previously shown image may still be visible
				// and `shown` is the only record of the area cleanup needs;
				// or it can fail after `shown_store()`, in which case the
				// new area is what cleanup must target. Either way the
				// truthful state is preserved for `image_hide()` and
				// `ClearInventory`.
				Err(e) if driver.failover_safe() => err = Some(e),
				Err(e) => return Err(e),
			}
		}
		Err(err.unwrap_or_else(|| anyhow::anyhow!("no terminal image renderer available")))
	}

	// Ordered candidates with the last successfully used driver first, where
	// it is still offered. Terminal protocols cannot acknowledge an
	// unsupported sequence (an ignored write still succeeds at the OS
	// level), so selection stays evidence-driven; this ordering only avoids
	// re-probing the chain after a helper fallback already proved itself.
	fn order(&self) -> Vec<Driver> {
		let mut chain = Drivers::candidates(&EMULATOR);
		if let Some(selected) = self.selected.get()
			&& let Some(i) = chain.iter().position(|d| *d == selected)
		{
			let driver = chain.remove(i);
			chain.insert(0, driver);
		}
		chain
	}

	pub fn image_hide(&self) -> Result<()> {
		let Some(area) = self.shown.replace(None) else { return Ok(()) };
		match self.selected.get() {
			Some(driver) => driver.image_erase(area),
			None => Ok(()),
		}
	}

	pub fn shown_area(&self) -> Option<Rect> { self.shown.get() }

	pub(super) fn shown_store(&self, area: Rect) { self.shown.set(Some(area)); }
}

#[cfg(test)]
mod tests {
	use std::sync::Once;

	use ratatui_core::layout::Rect;
	use yazi_emulator::EMULATOR;

	use super::Adapter;

	fn emulator() {
		static INIT: Once = Once::new();
		INIT.call_once(|| {
			EMULATOR.init(Default::default());
			EMULATOR.probe.complete();
		});
	}

	// A failed render must not discard cleanup tracking. The drivers fail a
	// missing file before `ADAPTOR.image_hide()` (decode/spawn error precedes
	// any terminal write), so whatever was shown before is still the truth
	// `image_hide()` and `ClearInventory` need. No terminal is touched: the
	// missing file fails before any escape sequence is written, whichever
	// fallback driver the host env selects.
	#[tokio::test]
	async fn failed_show_preserves_old_tracking() {
		emulator();

		let adapter = Adapter::default();
		let old = Rect { x: 1, y: 2, width: 10, height: 5 };
		adapter.shown_store(old);

		let result = adapter
			.image_show(
				"/definitely/not/a/yazi-test-image.png",
				Rect { x: 0, y: 0, width: 80, height: 24 },
			)
			.await;
		assert!(result.is_err());
		assert!(!result.unwrap_err().to_string().is_empty());

		assert_eq!(adapter.shown_area(), Some(old));
		assert!(adapter.selected.get().is_none());
	}
}

inventory::submit! {
	ClearInventory {
		clear: |area| {
			let overlap = area.intersection(ADAPTOR.shown.get()?);
			if overlap.area() == 0 {
				return None;
			}

			ADAPTOR.selected.get()?.image_erase(overlap).ok();
			ADAPTOR.collision.set(true);
			Some(overlap)
		},
	}
}
