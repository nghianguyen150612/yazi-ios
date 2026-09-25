use std::{
	borrow::Cow,
	ffi::OsStr,
	fs::File,
	io::{self, BufRead, BufReader, Write},
	os::unix::ffi::OsStrExt,
	path::{Path, PathBuf},
};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode, percent_encode};
use yazi_shim::path::PathExt;

// Encode characters not allowed in URI paths per Freedesktop Trash spec
const TRASH_PATH_ENCODE_SET: &AsciiSet = &CONTROLS
	.add(b' ')
	.add(b'"')
	.add(b'#')
	.add(b'<')
	.add(b'>')
	.add(b'?')
	.add(b'[')
	.add(b'\\')
	.add(b']')
	.add(b'^')
	.add(b'`')
	.add(b'{')
	.add(b'|')
	.add(b'}');

pub(super) struct TrashInfo {
	#[allow(dead_code)]
	pub(super) root:     PathBuf,
	pub(super) backing:  PathBuf,
	pub(super) original: PathBuf,
}

impl TrashInfo {
	pub(super) fn parse(info: &Path) -> io::Result<Self> {
		if info.extension() != Some(OsStr::new("trashinfo")) {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid trash info path"));
		}

		let root = info
			.parent()
			.filter(|p| p.file_name() == Some(OsStr::new("info")))
			.and_then(Path::parent)
			.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid trash info path"))?;

		let stem = info
			.file_stem()
			.filter(|&stem| stem != OsStr::new(".") && stem != OsStr::new(".."))
			.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid trash info path"))?;

		let original = Self::parse_original(info)?;
		if original.file_name().is_none() {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid original trash path"));
		}

		Ok(Self { root: root.to_owned(), backing: root.join("files").join(stem), original })
	}

	fn parse_original(info: &Path) -> io::Result<PathBuf> {
		let mut reader = BufReader::new(File::open(info)?);
		let mut line = Vec::new();

		reader.read_until(b'\n', &mut line)?;
		Self::trim_line(&mut line);
		if line != b"[Trash Info]" {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid trash info header"));
		}

		loop {
			line.clear();
			if reader.read_until(b'\n', &mut line)? == 0 {
				return Err(io::Error::new(io::ErrorKind::InvalidData, "trash info has no Path"));
			}

			Self::trim_line(&mut line);
			let Some(value) = line.strip_prefix(b"Path=") else { continue };
			let decoded: Cow<[u8]> = percent_decode(value).into();

			let path = Path::new(OsStr::from_bytes(decoded.as_ref()));
			if path.as_os_str().is_empty() || !path.is_absolute() || path.has_parent_component() {
				return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid original trash path"));
			}

			return Ok(path.to_owned());
		}
	}

	pub(super) fn write_info(info_path: &Path, original_path: &Path) -> io::Result<()> {
		let mut file = File::create(info_path)?;
		let encoded_path = percent_encode(original_path.as_os_str().as_bytes(), TRASH_PATH_ENCODE_SET).to_string();
		let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");

		writeln!(file, "[Trash Info]")?;
		writeln!(file, "Path={}", encoded_path)?;
		writeln!(file, "DeletionDate={}", now)?;
		file.flush()?;
		Ok(())
	}

	fn trim_line(line: &mut Vec<u8>) {
		while matches!(line.last(), Some(b'\n' | b'\r')) {
			line.pop();
		}
	}
}
