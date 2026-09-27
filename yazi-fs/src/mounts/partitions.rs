use std::ops::Deref;

use parking_lot::RwLock;
#[cfg(any(target_os = "ios", test))]
use yazi_shared::natsort;
use yazi_shim::cell::RoCell;

use super::Partition;
use crate::cha::Cha;

pub(super) type Locked = RwLock<Partitions>;

pub static PARTITIONS: RoCell<Locked> = RoCell::new();

#[derive(Default)]
pub struct Partitions {
	pub(super) inner:       Vec<Partition>,
	#[cfg(target_os = "linux")]
	pub(super) linux_cache: hashbrown::HashSet<String>,
	#[cfg(target_os = "macos")]
	pub(super) need_update: bool,
}

impl Deref for Partitions {
	type Target = Vec<Partition>;

	fn deref(&self) -> &Self::Target { &self.inner }
}

impl Partitions {
	#[cfg(unix)]
	fn by_dev(&self, dev: u64) -> Option<&Partition> {
		self.inner.iter().find(|p| p.rdev == Some(dev))
	}

	pub fn timeless(&self, _cha: Cha) -> bool {
		#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
		{
			self.by_dev(_cha.dev).is_some_and(|p| p.timeless())
		}
		#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "ios")))]
		{
			// For now, assume other targets update directory mtime correctly
			false
		}
	}

	pub fn soundless(&self, _cha: Cha) -> bool {
		#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
		{
			self.by_dev(_cha.dev).is_some_and(|p| p.soundless())
		}
		#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "ios")))]
		{
			// For now, assume other targets emit change notifications correctly
			false
		}
	}

	/// Deterministic mount-table ordering: by mount point, then source.
	#[cfg(any(target_os = "ios", test))]
	pub(super) fn sort_mounts(parts: &mut [Partition]) {
		parts.sort_unstable_by(|a, b| {
			natsort(Self::dist_bytes(a), Self::dist_bytes(b), false).then_with(|| {
				natsort(a.src.as_encoded_bytes(), b.src.as_encoded_bytes(), false)
			})
		});
	}

	/// True when two snapshots describe different mount tables.
	///
	/// Both snapshots must already be in [`Partitions::sort_mounts`] order.
	/// Only mount identity (source, mount point, filesystem type) is compared;
	/// volatile capacity counters are ignored so a filling filesystem does not
	/// look like a mount event.
	#[cfg(any(target_os = "ios", test))]
	pub(super) fn mounts_changed(before: &[Partition], after: &[Partition]) -> bool {
		before.len() != after.len()
			|| before
				.iter()
				.zip(after)
				.any(|(a, b)| a.src != b.src || a.dist != b.dist || a.fstype != b.fstype)
	}

	#[cfg(any(target_os = "ios", test))]
	fn dist_bytes(p: &Partition) -> &[u8] {
		p.dist.as_ref().map(|d| d.as_os_str().as_encoded_bytes()).unwrap_or(b"")
	}
}

#[cfg(test)]
mod tests {
	use super::{Partition, Partitions};

	fn snapshot() -> Vec<Partition> {
		let mut parts = vec![
			Partition::from_mount_record(
				b"/dev/disk1s2",
				b"/System/Volumes/Data",
				b"apfs",
				9,
				4096,
				Some(2),
			),
			Partition::from_mount_record(b"/dev/disk1s1", b"/", b"apfs", 7, 4096, Some(1)),
			Partition::from_mount_record(b"devfs", b"/dev", b"devfs", 1, 512, Some(3)),
		];
		Partitions::sort_mounts(&mut parts);
		parts
	}

	#[test]
	fn snapshot_ordering_is_deterministic() {
		let parts = snapshot();
		let dists: Vec<_> =
			parts.iter().filter_map(|p| p.dist.as_ref().and_then(|d| d.to_str())).collect();
		assert_eq!(dists, ["/", "/System/Volumes/Data", "/dev"]);
	}

	#[test]
	fn identical_snapshots_report_no_change() {
		assert!(!Partitions::mounts_changed(&snapshot(), &snapshot()));
	}

	#[test]
	fn added_and_removed_mounts_report_change() {
		let mut added = snapshot();
		added.push(Partition::from_mount_record(
			b"/dev/disk2s1",
			b"/Volumes/USB",
			b"exfat",
			5,
			4096,
			Some(4),
		));
		Partitions::sort_mounts(&mut added);
		assert!(Partitions::mounts_changed(&snapshot(), &added));

		let mut removed = snapshot();
		removed.pop();
		assert!(Partitions::mounts_changed(&snapshot(), &removed));
	}

	#[test]
	fn moved_mount_point_and_changed_type_report_change() {
		let before = snapshot();

		let mut moved = snapshot();
		moved[0] = Partition::from_mount_record(
			b"/dev/disk1s1",
			b"/Volumes/Other",
			b"apfs",
			7,
			4096,
			Some(1),
		);
		Partitions::sort_mounts(&mut moved);
		assert!(Partitions::mounts_changed(&before, &moved));

		let mut retyped = snapshot();
		retyped[1] = Partition::from_mount_record(
			b"/dev/disk1s2",
			b"/System/Volumes/Data",
			b"smbfs",
			9,
			4096,
			Some(2),
		);
		assert!(Partitions::mounts_changed(&before, &retyped));
	}

	#[test]
	fn free_space_only_change_reports_no_change() {
		let before = snapshot();
		let mut after = snapshot();
		after[1] = Partition::from_mount_record(
			b"/dev/disk1s2",
			b"/System/Volumes/Data",
			b"apfs",
			3,
			4096,
			Some(2),
		);
		assert!(!Partitions::mounts_changed(&before, &after));
	}

	#[cfg(unix)]
	#[test]
	fn device_lookup_associates_mounts_by_st_dev() {
		let partitions = Partitions { inner: snapshot(), ..Default::default() };
		assert_eq!(partitions.by_dev(1).map(|p| p.dist.clone().unwrap()), Some("/".into()));
		assert!(partitions.by_dev(999).is_none());
	}
}
