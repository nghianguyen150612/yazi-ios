use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, Default)]
pub struct Partition {
	pub src:         OsString,
	pub dist:        Option<PathBuf>,
	#[cfg(unix)]
	pub(crate) rdev: Option<u64>,
	pub label:       Option<OsString>,
	pub fstype:      Option<OsString>,
	pub capacity:    u64,
	pub external:    Option<bool>,
	pub removable:   Option<bool>,
}

impl Partition {
	/// Build a partition from raw BSD mount-table fields.
	///
	/// `src`, `dist`, and `fstype` are the raw `f_mntfromname`, `f_mntonname`,
	/// and `f_fstypename` bytes; trailing NUL padding is removed. `blocks` and
	/// `bsize` are `f_blocks` and `f_bsize`, and `rdev` is the `st_dev` of the
	/// mount point when it could be read.
	///
	/// Label, external, and removable metadata is not reported by the mount
	/// table and stays `None` rather than being fabricated. An empty
	/// filesystem type means the kernel reported none and is kept as `None`.
	#[cfg(any(target_os = "ios", test))]
	pub(super) fn from_mount_record(
		src: &[u8],
		dist: &[u8],
		fstype: &[u8],
		blocks: u64,
		bsize: u32,
		rdev: Option<u64>,
	) -> Self {
		let fstype = Self::trim_nul(fstype);
		let partition = Self {
			src:      Self::os_from_bytes(Self::trim_nul(src)),
			dist:     Some(Self::os_from_bytes(Self::trim_nul(dist)).into()),
			label:    None,
			fstype:   if fstype.is_empty() { None } else { Some(Self::os_from_bytes(fstype)) },
			capacity: blocks.saturating_mul(bsize as u64),
			external: None,
			removable: None,
			..Default::default()
		};
		#[cfg(unix)]
		let partition = Self { rdev, ..partition };
		#[cfg(not(unix))]
		let _ = rdev;
		partition
	}

	#[cfg(any(target_os = "ios", test))]
	fn trim_nul(b: &[u8]) -> &[u8] {
		match b.iter().position(|&c| c == 0) {
			Some(i) => &b[..i],
			None => b,
		}
	}

	#[cfg(all(unix, any(target_os = "ios", test)))]
	fn os_from_bytes(b: &[u8]) -> OsString {
		use std::os::unix::ffi::OsStringExt;
		OsString::from_vec(b.to_vec())
	}

	#[cfg(all(not(unix), any(target_os = "ios", test)))]
	fn os_from_bytes(b: &[u8]) -> OsString {
		String::from_utf8_lossy(b).into_owned().into()
	}
}

impl Partition {
	// Match mount types that do not update directory mtime on changes,
	// and should be refreshed frequently.
	pub(crate) fn timeless(&self) -> bool {
		let b: &[u8] = self.fstype.as_ref().map_or(b"", |s| s.as_encoded_bytes());
		matches!(b, b"exfat")
	}

	// Match mount types that do not reliably emit change notifications,
	// and should be polled for changes.
	pub(crate) fn soundless(&self) -> bool {
		let b: &[u8] = self.fstype.as_ref().map_or(b"", |s| s.as_encoded_bytes());
		matches!(b, b"fuse.rclone" | b"nfs4")
	}

	#[rustfmt::skip]
	pub fn systemic(&self) -> bool {
		let _b: &[u8] = self.fstype.as_ref().map_or(b"", |s| s.as_encoded_bytes());
		#[cfg(target_os = "linux")]
		{
			matches!(_b, b"autofs" | b"binfmt_misc" | b"bpf" | b"cgroup2" | b"configfs" | b"debugfs" | b"devpts" | b"devtmpfs" | b"fuse.gvfsd-fuse" | b"fusectl" | b"hugetlbfs" | b"mqueue" | b"proc" | b"pstore" | b"ramfs" | b"securityfs" | b"sysfs" | b"tmpfs" | b"tracefs")
		}
		#[cfg(target_os = "macos")]
		{
			_b.is_empty()
		}
		#[cfg(not(any(target_os = "linux", target_os = "macos")))]
		{
			false
		}
	}
}

impl Partition {
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	pub(super) fn new(name: &std::ffi::OsStr) -> Self {
		Self { src: std::path::Path::new("/dev/").join(name).into(), ..Default::default() }
	}

	#[cfg(target_os = "linux")]
	pub(super) fn dev_name(&self, full: bool) -> Option<&std::ffi::OsStr> {
		use std::os::unix::ffi::OsStrExt;

		let s = std::path::Path::new(&self.src).strip_prefix("/dev/").ok()?.as_os_str();
		if full {
			return Some(s);
		}

		let b = s.as_bytes();
		if b.len() < 3 {
			None
		} else if b.starts_with(b"sd") || b.starts_with(b"hd") || b.starts_with(b"vd") {
			Some(std::ffi::OsStr::from_bytes(&b[..3]))
		} else if b.starts_with(b"nvme") || b.starts_with(b"mmcblk") {
			let n = b.iter().position(|&b| b == b'p').unwrap_or(b.len());
			Some(std::ffi::OsStr::from_bytes(&b[..n]))
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::Partition;

	fn record() -> Partition {
		Partition::from_mount_record(
			b"/dev/disk1s1\0\0\0",
			b"/System/Volumes/Data\0",
			b"apfs\0\0\0\0\0\0\0\0\0\0\0\0",
			1000,
			4096,
			Some(42),
		)
	}

	#[test]
	fn mount_record_maps_source_destination_and_type() {
		let p = record();
		assert_eq!(p.src.as_encoded_bytes(), b"/dev/disk1s1");
		assert_eq!(p.dist, Some(PathBuf::from("/System/Volumes/Data")));
		assert_eq!(p.fstype.as_ref().map(|s| s.as_encoded_bytes()), Some(b"apfs".as_slice()));
		assert_eq!(p.capacity, 1000 * 4096);
		#[cfg(unix)]
		assert_eq!(p.rdev, Some(42));
	}

	#[test]
	fn mount_record_keeps_unavailable_metadata_optional() {
		let p = record();
		assert_eq!(p.label, None);
		assert_eq!(p.external, None);
		assert_eq!(p.removable, None);
	}

	#[test]
	fn mount_record_maps_missing_fstype_to_none() {
		let p = Partition::from_mount_record(b"devfs", b"/dev", b"\0\0\0", 1, 512, None);
		assert_eq!(p.fstype, None);
	}

	#[test]
	fn mount_record_capacity_saturates_on_overflow() {
		let p = Partition::from_mount_record(b"s", b"/m", b"apfs", u64::MAX, u32::MAX, None);
		assert_eq!(p.capacity, u64::MAX);
	}

	#[cfg(unix)]
	#[test]
	fn mount_record_preserves_non_utf8_bytes() {
		let p = Partition::from_mount_record(b"/dev/\xff\xfe", b"/mnt/\xff", b"msdos", 1, 512, None);
		assert_eq!(p.src.as_encoded_bytes(), b"/dev/\xff\xfe");
		assert_eq!(p.dist.unwrap().as_os_str().as_encoded_bytes(), b"/mnt/\xff");
		assert_eq!(p.fstype.unwrap().as_encoded_bytes(), b"msdos");
	}

	#[test]
	fn timeless_filesystem_classification() {
		assert!(Partition::from_mount_record(b"s", b"/m", b"exfat", 1, 512, None).timeless());
		assert!(!record().timeless());
		assert!(!Partition::from_mount_record(b"s", b"/m", b"", 1, 512, None).timeless());
	}

	#[test]
	fn soundless_filesystem_classification() {
		for fs in [b"fuse.rclone".as_slice(), b"nfs4"] {
			assert!(Partition::from_mount_record(b"s", b"/m", fs, 1, 512, None).soundless());
		}
		for fs in [b"apfs".as_slice(), b"smbfs", b"nfs", b""] {
			assert!(!Partition::from_mount_record(b"s", b"/m", fs, 1, 512, None).soundless());
		}
	}
}
