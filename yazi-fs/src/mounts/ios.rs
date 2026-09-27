use std::{ffi::OsStr, os::unix::{ffi::OsStrExt, fs::MetadataExt}, time::Duration};

use anyhow::{Context, Result};
use tokio::time::sleep;
use yazi_macro::error;

use super::{Locked, Partition, Partitions};

/// Interval between mount-table snapshot comparisons.
///
/// Mount changes are rare; this only needs to notice a volume appearing or
/// disappearing within a reasonable time. It must not wake the process
/// continuously or hold any file descriptors between polls.
const POLL_INTERVAL: Duration = Duration::from_secs(10);

impl Partitions {
	pub fn monitor<F>(me: &'static Locked, cb: F)
	where
		F: Fn() + Copy + Send + 'static,
	{
		async fn refresh(me: &'static Locked) -> Result<bool> {
			let next = tokio::task::spawn_blocking(Partitions::enumerate).await??;
			let mut guard = me.write();
			if Partitions::mounts_changed(&guard.inner, &next) {
				guard.inner = next;
				Ok(true)
			} else {
				Ok(false)
			}
		}

		tokio::spawn(async move {
			// The first refresh populates PARTITIONS from its default empty
			// state, so Yazi never needs a real mount event after startup
			// before the snapshot becomes useful.
			loop {
				match refresh(me).await {
					Ok(true) => cb(),
					Ok(false) => {}
					Err(e) => error!("Error encountered while updating mount points: {e:?}"),
				}
				sleep(POLL_INTERVAL).await;
			}
		});
	}

	fn enumerate() -> Result<Vec<Partition>> {
		// Ask the kernel how many mounts exist, then fetch them. The table can
		// grow between the two calls, so leave slack and retry if it did.
		let mut buf: Vec<libc::statfs> = Vec::new();
		let mut filled = 0;
		for _ in 0..3 {
			let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
			if count < 0 {
				return Err(std::io::Error::last_os_error()).context("getfsstat: count mounts");
			}

			buf.clear();
			buf.resize_with((count as usize).saturating_add(8), || unsafe { std::mem::zeroed() });
			let got = unsafe {
				libc::getfsstat(
					buf.as_mut_ptr(),
					(buf.len() * std::mem::size_of::<libc::statfs>()) as libc::c_int,
					libc::MNT_NOWAIT,
				)
			};
			if got < 0 {
				return Err(std::io::Error::last_os_error()).context("getfsstat: list mounts");
			}
			filled = (got as usize).min(buf.len());
			if (got as usize) <= buf.len() {
				break;
			}
		}

		buf.truncate(filled);
		let mut parts: Vec<_> = buf.iter().map(Self::convert).collect();
		Partitions::sort_mounts(&mut parts);
		Ok(parts)
	}

	fn convert(sf: &libc::statfs) -> Partition {
		let src = Self::field(&sf.f_mntfromname);
		let dist = Self::field(&sf.f_mntonname);

		// Associate the mount with the st_dev of its mount point, which is the
		// same st_dev that Cha reports for every file on that filesystem. A
		// mount point that cannot be statted still yields a useful record; it
		// only opts out of by_dev classification.
		let rdev = std::fs::metadata(OsStr::from_bytes(dist)).ok().map(|m| m.dev());

		Partition::from_mount_record(
			src,
			dist,
			Self::field(&sf.f_fstypename),
			sf.f_blocks,
			sf.f_bsize,
			rdev,
		)
	}

	// Raw bytes of a NUL-terminated fixed-size C string field.
	fn field(arr: &[libc::c_char]) -> &[u8] {
		let bytes = unsafe { std::slice::from_raw_parts(arr.as_ptr().cast::<u8>(), arr.len()) };
		match bytes.iter().position(|&b| b == 0) {
			Some(i) => &bytes[..i],
			None => bytes,
		}
	}
}
