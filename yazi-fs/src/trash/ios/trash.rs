use std::{
	ffi::OsStr,
	fs,
	hash::Hash,
	io,
	path::{Path, PathBuf},
};

use yazi_macro::ok_or_not_found;
use yazi_shim::Twox128;

use super::{
	TrashInfo,
	super::{TrashCha, TrashEntries, TrashEntry, TrashId, restore_item},
};
use crate::{
	cha::{Cha, ChaSig},
	file::File,
};

pub struct Trash;

#[allow(dead_code)]
impl Trash {
	pub(crate) fn new() -> io::Result<Self> { Ok(Self) }

	pub(crate) fn trash_root(&self) -> io::Result<PathBuf> {
		#[cfg(test)]
		if let Ok(test_root) = std::env::var("YAZI_TEST_TRASH_ROOT") {
			let path = PathBuf::from(test_root);
			fs::create_dir_all(path.join("files"))?;
			fs::create_dir_all(path.join("info"))?;
			return Ok(path);
		}

		let home = dirs::home_dir()
			.filter(|p| p.is_absolute())
			.ok_or_else(|| io::Error::other("cannot determine home directory for trash root resolution"))?;

		let root = home.join(".local/share/Trash");
		fs::create_dir_all(root.join("files"))?;
		fs::create_dir_all(root.join("info"))?;
		Ok(root)
	}

	pub(crate) fn list(&self, entry: Option<&TrashEntry>) -> io::Result<Vec<TrashEntry>> {
		let Some(entry) = entry else {
			return self.tops();
		};

		if !entry.lcha.is_dir() {
			return Err(io::Error::new(io::ErrorKind::InvalidInput, "trash item is not a directory"));
		}

		fs::read_dir(&entry.backing)?
			.map(|dent| {
				let dent = dent?;
				entry.child(dent.file_name())
			})
			.collect()
	}

	pub(crate) fn entry(&self, id: &TrashId) -> io::Result<TrashEntry> {
		let info = TrashInfo::parse(id.top())?;
		let root = self.trash_root()?;
		if info.root != root {
			return Err(io::Error::new(io::ErrorKind::NotFound, "trash item outside of trash folders"));
		}

		if id.has_rel() && !fs::symlink_metadata(&info.backing)?.file_type().is_dir() {
			return Err(io::Error::new(io::ErrorKind::InvalidInput, "trash item is not a directory"));
		}

		let (backing, original) = if id.has_rel() {
			(info.backing.join(id.rel()), info.original.join(id.rel()))
		} else {
			(info.backing, info.original)
		};
		TrashEntry::new(id.clone(), backing, Some(original))
	}

	pub(crate) fn metadata(&self, entry: &TrashEntry, follow: bool) -> io::Result<Cha> {
		Ok(if follow { entry.cha } else { entry.lcha })
	}

	pub(crate) fn revalidate(
		&self,
		entry: Option<&TrashEntry>,
		current: &File,
	) -> io::Result<Option<File>> {
		let latest = if let Some(entry) = entry {
			entry.clone().into_file(&current.url)
		} else {
			let root = self.trash_root()?;
			let mut h = Twox128::default();
			if let Ok(meta) = fs::metadata(root.join("info")) {
				let cha = Cha::new(root.file_name().unwrap_or_default(), meta);
				root.hash(&mut h);
				ChaSig(cha).hash(&mut h);
			}

			let hash = h.finish_128();
			File {
				cha: Cha { len: hash as u64 ^ (hash >> 64) as u64, ..Cha::from_mold(true) },
				..current.clone()
			}
		};

		let changed = !latest.cha.hits(current.cha)
			|| latest.extra.link_to() != current.extra.link_to()
			|| latest.extra.backing() != current.extra.backing();

		Ok(changed.then_some(latest))
	}

	pub(crate) fn remove_file(&self, entry: &TrashEntry) -> io::Result<()> {
		fs::remove_file(&entry.backing)?;
		if !entry.has_rel() {
			fs::remove_file(entry.top())?;
		}
		Ok(())
	}

	pub(crate) fn remove_dir(&self, entry: &TrashEntry) -> io::Result<()> {
		fs::remove_dir(&entry.backing)?;
		if !entry.has_rel() {
			fs::remove_file(entry.top())?;
		}
		Ok(())
	}

	pub(crate) fn restore(&self, entries: TrashEntries) -> io::Result<()> {
		for entry in entries {
			let to = entry.original.clone().ok_or_else(|| {
				io::Error::new(io::ErrorKind::NotFound, "trash item has no put-back location")
			})?;

			restore_item(&entry.backing, &to)?;

			if !entry.has_rel() {
				fs::remove_file(entry.top())?;
			}
		}
		Ok(())
	}

	pub(crate) fn rename(&self, entry: &TrashEntry, path: &Path) -> io::Result<()> {
		fs::rename(&entry.backing, path)
	}

	pub(crate) fn empty(&self) -> io::Result<()> {
		for entry in self.tops()? {
			if entry.lcha.is_dir() {
				fs::remove_dir_all(&entry.backing)?;
			} else {
				fs::remove_file(&entry.backing)?;
			}
			fs::remove_file(entry.top())?;
		}
		Ok(())
	}

	pub(super) fn tops(&self) -> io::Result<Vec<TrashEntry>> {
		let mut tops = Vec::new();
		let root = self.trash_root()?;
		for dent in ok_or_not_found!(fs::read_dir(root.join("info")), return Ok(vec![])) {
			let dent = dent?;
			let info = dent.path();
			if let Ok(parsed) = TrashInfo::parse(&info) {
				tops.push(ok_or_not_found!(
					TrashEntry::top(info, parsed.backing, Some(parsed.original)),
					continue
				));
			}
		}
		Ok(tops)
	}

	pub fn move_to_trash(&self, path: &Path) -> io::Result<()> {
		let root = self.trash_root()?;
		let files_dir = root.join("files");
		let info_dir = root.join("info");

		let file_name = path
			.file_name()
			.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no filename"))?;

		// Handle filename collisions in files_dir and info_dir
		let unique_stem = Self::find_unique_trash_name(&files_dir, &info_dir, file_name)?;

		let info_file = info_dir.join(format!("{}.trashinfo", unique_stem.to_string_lossy()));
		let target_file = files_dir.join(&unique_stem);

		// Write trash info first
		let canonical_original = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
		TrashInfo::write_info(&info_file, &canonical_original)?;

		// Try moving payload into files_dir; if cross-filesystem rename fails, copy and remove
		if let Err(err) = fs::rename(path, &target_file) {
			if err.kind() == io::ErrorKind::CrossesDevices || err.raw_os_error() == Some(libc::EXDEV) {
				Self::copy_and_remove(path, &target_file)?;
			} else {
				let _ = fs::remove_file(&info_file);
				return Err(err);
			}
		}

		Ok(())
	}

	fn find_unique_trash_name(files_dir: &Path, info_dir: &Path, file_name: &OsStr) -> io::Result<PathBuf> {
		let name_path = Path::new(file_name);
		let stem = name_path.file_stem().unwrap_or(file_name).to_string_lossy();
		let ext = name_path.extension().map(|e| e.to_string_lossy());

		let mut count = 0usize;
		loop {
			let candidate_name = if count == 0 {
				file_name.to_os_string()
			} else if let Some(ref ext_str) = ext {
				format!("{} {}.{}", stem, count, ext_str).into()
			} else {
				format!("{} {}", stem, count).into()
			};

			let candidate_path = PathBuf::from(&candidate_name);
			let info_candidate = info_dir.join(format!("{}.trashinfo", candidate_name.to_string_lossy()));
			let file_candidate = files_dir.join(&candidate_path);

			if !info_candidate.exists() && !file_candidate.exists() {
				return Ok(candidate_path);
			}

			count += 1;
		}
	}

	fn copy_and_remove(src: &Path, dst: &Path) -> io::Result<()> {
		let meta = fs::symlink_metadata(src)?;
		if meta.file_type().is_symlink() {
			let target = fs::read_link(src)?;
			std::os::unix::fs::symlink(target, dst)?;
			fs::remove_file(src)?;
		} else if meta.is_dir() {
			fs::create_dir_all(dst)?;
			for entry in fs::read_dir(src)? {
				let entry = entry?;
				Self::copy_and_remove(&entry.path(), &dst.join(entry.file_name()))?;
			}
			fs::remove_dir(src)?;
		} else {
			fs::copy(src, dst)?;
			fs::remove_file(src)?;
		}
		Ok(())
	}
}
