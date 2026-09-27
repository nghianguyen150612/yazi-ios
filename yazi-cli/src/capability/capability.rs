use std::{
	collections::BTreeMap,
	ffi::{OsStr, OsString},
	fmt::{self, Write},
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityClass {
	Core,
	Feature,
	Fallback,
	Integration,
	Configured,
	NotApplicable,
}

impl fmt::Display for CapabilityClass {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Core => "core",
			Self::Feature => "feature",
			Self::Fallback => "fallback",
			Self::Integration => "integration",
			Self::Configured => "configured",
			Self::NotApplicable => "not_applicable",
		})
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityStatus {
	Available,
	Missing,
	NotApplicable,
	Configured,
	Unknown,
}

impl CapabilityStatus {
	fn label(self) -> &'static str {
		match self {
			Self::Available => "OK",
			Self::Missing => "OPTIONAL MISSING",
			Self::NotApplicable => "NOT APPLICABLE",
			Self::Configured => "CONFIGURED",
			Self::Unknown => "UNKNOWN",
		}
	}
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Capability {
	pub(crate) name: String,
	pub(crate) group: String,
	pub(crate) class: CapabilityClass,
	pub(crate) feature: String,
	pub(crate) status: CapabilityStatus,
	pub(crate) candidates: Vec<String>,
	pub(crate) selected: Option<String>,
	pub(crate) version: Option<String>,
	pub(crate) override_env: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProbeContext {
	pub(crate) path: Option<OsString>,
	pub(crate) file_one: Option<OsString>,
	pub(crate) editor: Option<OsString>,
	pub(crate) shell: Option<OsString>,
}

impl ProbeContext {
	pub(crate) fn from_env() -> Self {
		Self {
			path: std::env::var_os("PATH"),
			file_one: std::env::var_os("YAZI_FILE_ONE"),
			editor: std::env::var_os("EDITOR"),
			shell: std::env::var_os("SHELL"),
		}
	}

	#[cfg(test)]
	pub(crate) fn with_path(path: impl Into<OsString>) -> Self {
		Self { path: Some(path.into()), file_one: None, editor: None, shell: None }
	}
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CapabilityReport {
	pub(crate) target_os: String,
	pub(crate) target_family: String,
	pub(crate) path: Option<String>,
	pub(crate) capabilities: Vec<Capability>,
}

impl CapabilityReport {
	pub(crate) fn collect() -> Self {
		Self::collect_with(ProbeContext::from_env())
	}

	pub(crate) fn collect_with(context: ProbeContext) -> Self {
		let mut capabilities = Vec::new();
		capabilities.push(Capability {
			name: "core-startup".to_owned(),
			group: "Core".to_owned(),
			class: CapabilityClass::Core,
			feature: "basic Yazi startup".to_owned(),
			status: CapabilityStatus::Available,
			candidates: Vec::new(),
			selected: None,
			version: None,
			override_env: None,
		});

		push_alias(
			&mut capabilities,
			"fd",
			"Search",
			"filename search",
			CapabilityClass::Feature,
			["fd", "fdfind"],
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"rg",
			"Search",
			"content search",
			CapabilityClass::Feature,
			"rg",
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"fzf",
			"Search",
			"interactive jump",
			CapabilityClass::Feature,
			"fzf",
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"zoxide",
			"Search",
			"directory history jump",
			CapabilityClass::Feature,
			"zoxide",
			&["--version"],
			&context,
			None,
		);

		push_effective(
			&mut capabilities,
			"file",
			"Preview",
			"MIME detection and generic file preview",
			CapabilityClass::Feature,
			context.file_one.as_deref(),
			"file",
			&["--version"],
			&context,
		);
		push_single(
			&mut capabilities,
			"jq",
			"Preview",
			"JSON preview",
			CapabilityClass::Fallback,
			"jq",
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"ffmpeg",
			"Preview",
			"video preview frames",
			CapabilityClass::Feature,
			"ffmpeg",
			&["-version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"ffprobe",
			"Preview",
			"video metadata",
			CapabilityClass::Feature,
			"ffprobe",
			&["-version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"pdftoppm",
			"Preview",
			"PDF image preview",
			CapabilityClass::Feature,
			"pdftoppm",
			&["--help"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"magick",
			"Preview",
			"ImageMagick image preview",
			CapabilityClass::Feature,
			"magick",
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"resvg",
			"Preview",
			"SVG rendered preview",
			CapabilityClass::Feature,
			"resvg",
			&["--version"],
			&context,
			None,
		);
		push_alias(
			&mut capabilities,
			"7zip",
			"Preview",
			"archive listing and extraction",
			CapabilityClass::Feature,
			["7zz", "7z"],
			&["i"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"chafa",
			"Preview",
			"terminal-native image fallback",
			CapabilityClass::Fallback,
			"chafa",
			&["--version"],
			&context,
			None,
		);
		push_platform(
			&mut capabilities,
			"ueberzugpp",
			"Preview",
			"desktop compositor image preview",
			CapabilityClass::Integration,
			"ueberzugpp",
			&["--version"],
			&context,
			std::env::consts::OS == "ios",
		);

		push_single(
			&mut capabilities,
			"git",
			"Packages",
			"ya pkg operations",
			CapabilityClass::Feature,
			"git",
			&["--version"],
			&context,
			None,
		);

		push_platform(
			&mut capabilities,
			"tmux",
			"Terminal",
			"tmux passthrough and session integration",
			CapabilityClass::Integration,
			"tmux",
			&["-V"],
			&context,
			false,
		);
		push_platform(
			&mut capabilities,
			"zellij",
			"Terminal",
			"zellij session integration",
			CapabilityClass::Integration,
			"zellij",
			&["--version"],
			&context,
			false,
		);

		for (name, feature, candidates, args) in [
			("pbcopy", "desktop clipboard writer", vec!["pbcopy"], vec!["--version"]),
			("pbpaste", "desktop clipboard reader", vec!["pbpaste"], vec!["--version"]),
			("termux-clipboard-set", "Android clipboard writer", vec!["termux-clipboard-set"], vec![]),
			("termux-clipboard-get", "Android clipboard reader", vec!["termux-clipboard-get"], vec![]),
			("wl-copy", "Wayland clipboard writer", vec!["wl-copy"], vec!["--version"]),
			("wl-paste", "Wayland clipboard reader", vec!["wl-paste"], vec!["--version"]),
			("xclip", "X11 clipboard", vec!["xclip"], vec!["-version"]),
			("xsel", "X11 clipboard", vec!["xsel"], vec!["--version"]),
		] {
			push_platform(
				&mut capabilities,
				name,
				"Clipboard",
				feature,
				CapabilityClass::Integration,
				candidates[0],
				&args,
				&context,
				std::env::consts::OS == "ios",
			);
		}
		push_native_ios(
			&mut capabilities,
			"ios-native-clipboard",
			"Clipboard",
			"native iOS clipboard",
			std::env::consts::OS == "ios",
		);

		let editor = context.editor.as_deref().unwrap_or(OsStr::new("vi"));
		push_configured(
			&mut capabilities,
			"editor",
			"Configured commands",
			"editor opener",
			editor,
			&context,
			context.editor.is_some(),
		);
		let shell = context.shell.as_deref().unwrap_or(if cfg!(windows) {
			OsStr::new("cmd.exe")
		} else {
			OsStr::new("sh")
		});
		push_configured(
			&mut capabilities,
			"shell",
			"Configured commands",
			"shell action",
			shell,
			&context,
			context.shell.is_some(),
		);
		push_single(
			&mut capabilities,
			"mediainfo",
			"Configured commands",
			"user-selected media information opener",
			CapabilityClass::Configured,
			"mediainfo",
			&["--version"],
			&context,
			None,
		);
		push_single(
			&mut capabilities,
			"exiftool",
			"Configured commands",
			"user-selected metadata opener",
			CapabilityClass::Configured,
			"exiftool",
			&["-ver"],
			&context,
			None,
		);
		let opener = match std::env::consts::OS {
			"linux" => Some(("xdg-open", "Linux system opener")),
			"macos" => Some(("open", "macOS system opener")),
			"android" => Some(("termux-open", "Android system opener")),
			_ => None,
		};
		if let Some((candidate, feature)) = opener {
			push_single(
				&mut capabilities,
				"system-opener",
				"Configured commands",
				feature,
				CapabilityClass::Configured,
				candidate,
				&["--version"],
				&context,
				None,
			);
		} else {
			capabilities.push(Capability {
				name: "system-opener".to_owned(),
				group: "Configured commands".to_owned(),
				class: CapabilityClass::NotApplicable,
				feature: "desktop/Android system opener".to_owned(),
				status: CapabilityStatus::NotApplicable,
				candidates: vec!["xdg-open".to_owned(), "open".to_owned(), "termux-open".to_owned()],
				selected: None,
				version: None,
				override_env: None,
			});
		}
		push_native_ios(
			&mut capabilities,
			"ios-native-opener",
			"Configured commands",
			"native iOS application handoff",
			std::env::consts::OS == "ios",
		);

		CapabilityReport {
			target_os: std::env::consts::OS.to_owned(),
			target_family: std::env::consts::FAMILY.to_owned(),
			path: context.path.map(|path| path.to_string_lossy().into_owned()),
			capabilities,
		}
	}

	pub(crate) fn render(&self) -> String {
		let mut output = String::new();
		writeln!(output, "Yazi external capabilities").ok();
		writeln!(output, "Target: {} ({})", self.target_os, self.target_family).ok();
		writeln!(output, "PATH: {}", self.path.as_deref().unwrap_or("<unset>")).ok();
		let mut groups = BTreeMap::<&str, Vec<&Capability>>::new();
		for capability in &self.capabilities {
			groups.entry(&capability.group).or_default().push(capability);
		}
		for (group, capabilities) in groups {
			writeln!(output, "\n{group}").ok();
			for capability in capabilities {
				let selected =
					capability.selected.as_deref().map(|name| format!(" [{name}]")).unwrap_or_default();
				let version =
					capability.version.as_deref().map(|version| format!(" — {version}")).unwrap_or_default();
				writeln!(
					output,
					"  {:16} {:10} {}{}{}",
					capability.name,
					capability.status.label(),
					capability.feature,
					selected,
					version
				)
				.ok();
			}
		}
		output
	}

	pub(crate) fn env_lines(&self) -> Vec<String> {
		self
			.capabilities
			.iter()
			.filter(|capability| capability.group != "Core")
			.map(|capability| {
				let selected = capability.selected.as_deref().unwrap_or("-");
				let version = capability.version.as_deref().unwrap_or(capability.status.label());
				format!("{:16}: {version} ({selected})", capability.name)
			})
			.collect()
	}

	pub(crate) fn value(&self, name: &str) -> String {
		self
			.capabilities
			.iter()
			.find(|capability| capability.name == name)
			.map(|capability| {
				capability.version.as_deref().unwrap_or(capability.status.label()).to_owned()
			})
			.unwrap_or_else(|| "UNKNOWN".to_owned())
	}
}

fn push_single(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	class: CapabilityClass,
	candidate: &str,
	args: &[&str],
	context: &ProbeContext,
	override_env: Option<&str>,
) {
	push_candidates(
		capabilities,
		name,
		group,
		feature,
		class,
		vec![OsString::from(candidate)],
		args,
		context,
		override_env,
		false,
	);
}

fn push_alias<const N: usize>(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	class: CapabilityClass,
	candidates: [&str; N],
	args: &[&str],
	context: &ProbeContext,
	override_env: Option<&str>,
) {
	push_candidates(
		capabilities,
		name,
		group,
		feature,
		class,
		candidates.into_iter().map(OsString::from).collect(),
		args,
		context,
		override_env,
		false,
	);
}

fn push_effective(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	class: CapabilityClass,
	override_arg: Option<&OsStr>,
	default: &str,
	args: &[&str],
	context: &ProbeContext,
) {
	let candidate = override_arg.unwrap_or(OsStr::new(default));
	push_candidates(
		capabilities,
		name,
		group,
		feature,
		class,
		vec![candidate.to_owned()],
		args,
		context,
		Some("YAZI_FILE_ONE"),
		false,
	);
}

fn push_platform(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	class: CapabilityClass,
	candidate: &str,
	args: &[&str],
	context: &ProbeContext,
	not_applicable: bool,
) {
	if not_applicable {
		capabilities.push(Capability {
			name: name.to_owned(),
			group: group.to_owned(),
			class: CapabilityClass::NotApplicable,
			feature: feature.to_owned(),
			status: CapabilityStatus::NotApplicable,
			candidates: vec![candidate.to_owned()],
			selected: None,
			version: None,
			override_env: None,
		});
	} else {
		push_single(capabilities, name, group, feature, class, candidate, args, context, None);
	}
}

fn push_native_ios(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	available: bool,
) {
	capabilities.push(Capability {
		name: name.to_owned(),
		group: group.to_owned(),
		class: if available { CapabilityClass::Integration } else { CapabilityClass::NotApplicable },
		feature: feature.to_owned(),
		status: if available { CapabilityStatus::Available } else { CapabilityStatus::NotApplicable },
		candidates: Vec::new(),
		selected: None,
		version: None,
		override_env: None,
	});
}

fn push_configured(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	value: &OsStr,
	context: &ProbeContext,
	configured: bool,
) {
	let selected = value.to_string_lossy().into_owned();
	let path_like = !configured && value == OsStr::new("vi")
		|| !configured && value == OsStr::new("sh")
		|| !configured && value == OsStr::new("cmd.exe");
	let resolved = path_like.then(|| resolve_executable(value, context.path.as_deref())).flatten();
	capabilities.push(Capability {
		name: name.to_owned(),
		group: group.to_owned(),
		class: CapabilityClass::Configured,
		feature: feature.to_owned(),
		status: if configured && value.is_empty() {
			CapabilityStatus::Unknown
		} else if configured {
			CapabilityStatus::Configured
		} else if resolved.is_some() {
			CapabilityStatus::Available
		} else {
			CapabilityStatus::Missing
		},
		candidates: vec![selected.clone()],
		selected: Some(selected),
		version: None,
		override_env: configured
			.then(|| if name == "editor" { "EDITOR/VISUAL" } else { "SHELL" }.to_owned()),
	});
}

fn push_candidates(
	capabilities: &mut Vec<Capability>,
	name: &str,
	group: &str,
	feature: &str,
	class: CapabilityClass,
	candidates: Vec<OsString>,
	args: &[&str],
	context: &ProbeContext,
	override_env: Option<&str>,
	not_applicable: bool,
) {
	let candidate_names =
		candidates.iter().map(|candidate| candidate.to_string_lossy().into_owned()).collect::<Vec<_>>();
	if not_applicable {
		capabilities.push(Capability {
			name: name.to_owned(),
			group: group.to_owned(),
			class: CapabilityClass::NotApplicable,
			feature: feature.to_owned(),
			status: CapabilityStatus::NotApplicable,
			candidates: candidate_names,
			selected: None,
			version: None,
			override_env: override_env.map(str::to_owned),
		});
		return;
	}

	let selected = candidates.iter().find_map(|candidate| {
		resolve_executable(candidate, context.path.as_deref()).map(|_| candidate.to_owned())
	});
	let version = selected
		.as_deref()
		.and_then(|candidate| resolve_executable(candidate, context.path.as_deref()))
		.and_then(|path| version_probe(&path, args));
	capabilities.push(Capability {
		name: name.to_owned(),
		group: group.to_owned(),
		class,
		feature: feature.to_owned(),
		status: if selected.is_some() {
			CapabilityStatus::Available
		} else {
			CapabilityStatus::Missing
		},
		candidates: candidate_names,
		selected: selected.map(|candidate| candidate.to_string_lossy().into_owned()),
		version,
		override_env: override_env.map(str::to_owned),
	});
}

fn version_probe(path: &Path, args: &[&str]) -> Option<String> {
	let output = Command::new(path)
		.args(args)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.ok()?;
	let bytes = if output.stdout.is_empty() { output.stderr } else { output.stdout };
	bytes
		.split(|byte| *byte == b'\n' || *byte == b'\r')
		.map(|line| String::from_utf8_lossy(line).trim().to_owned())
		.find(|line| !line.is_empty())
}

pub(crate) fn resolve_executable(candidate: &OsStr, path: Option<&OsStr>) -> Option<PathBuf> {
	if candidate.is_empty() {
		return None;
	}
	let candidate_path = Path::new(candidate);
	if candidate_path.is_absolute()
		|| candidate_path.parent().is_some_and(|parent| !parent.as_os_str().is_empty())
	{
		return executable_variant(candidate_path);
	}

	let path = path.filter(|path| !path.is_empty())?;
	for directory in std::env::split_paths(path) {
		let directory = if directory.as_os_str().is_empty() { Path::new(".") } else { &directory };
		let candidate_path = directory.join(candidate);
		if let Some(path) = executable_variant(&candidate_path) {
			return Some(path);
		}
	}
	None
}

fn executable_variant(path: &Path) -> Option<PathBuf> {
	#[cfg(windows)]
	{
		let mut variants = vec![path.to_owned()];
		if path.extension().is_none() {
			let pathext =
				std::env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
			for extension in pathext.to_string_lossy().split(';') {
				variants.push(path.with_extension(extension.trim_start_matches('.')));
			}
		}
		return variants.into_iter().find(|candidate| candidate.is_file());
	}

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		return path
			.metadata()
			.ok()
			.filter(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
			.map(|_| path.to_owned());
	}

	#[cfg(not(any(unix, windows)))]
	path.is_file().then(|| path.to_owned())
}

#[cfg(test)]
mod tests {
	#[cfg(unix)]
	use std::os::unix::fs::PermissionsExt;
	use std::{
		ffi::OsString,
		fs::{self, File},
		path::PathBuf,
	};

	use super::*;

	fn temp_dir(label: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("yazi-task017-{label}-{}", std::process::id()));
		fs::create_dir_all(&path).unwrap();
		path
	}

	fn executable(path: &Path) {
		File::create(path).unwrap();
		#[cfg(unix)]
		{
			let mut permissions = fs::metadata(path).unwrap().permissions();
			permissions.set_mode(0o755);
			fs::set_permissions(path, permissions).unwrap();
		}
	}

	#[test]
	fn empty_and_missing_path_do_not_resolve() {
		assert!(resolve_executable(OsStr::new("rg"), Some(OsStr::new(""))).is_none());
		assert!(resolve_executable(OsStr::new("rg"), Some(OsStr::new("/does/not/exist"))).is_none());
		assert!(resolve_executable(OsStr::new("rg"), None).is_none());
	}

	#[test]
	fn aliases_prefer_the_first_available_candidate() {
		let directory = temp_dir("aliases");
		executable(&directory.join("fd"));
		executable(&directory.join("fdfind"));
		executable(&directory.join("7z"));
		let context = ProbeContext::with_path(directory.as_os_str());
		let report = CapabilityReport::collect_with(context);
		let fd = report.capabilities.iter().find(|capability| capability.name == "fd").unwrap();
		assert_eq!(fd.status, CapabilityStatus::Available);
		assert_eq!(fd.selected.as_deref(), Some("fd"));
		let archive = report.capabilities.iter().find(|capability| capability.name == "7zip").unwrap();
		assert_eq!(archive.selected.as_deref(), Some("7z"));
	}

	#[test]
	fn aliases_fall_back_to_the_second_available_candidate() {
		let directory = temp_dir("alias-fallback");
		executable(&directory.join("fdfind"));
		executable(&directory.join("7z"));
		let report = CapabilityReport::collect_with(ProbeContext::with_path(directory.as_os_str()));
		let fd = report.capabilities.iter().find(|capability| capability.name == "fd").unwrap();
		assert_eq!(fd.status, CapabilityStatus::Available);
		assert_eq!(fd.selected.as_deref(), Some("fdfind"));
	}

	#[test]
	fn override_replaces_the_default_file_candidate() {
		let directory = temp_dir("file-override");
		executable(&directory.join("custom-file"));
		let mut context = ProbeContext::with_path(directory.as_os_str());
		context.file_one = Some(OsString::from("custom-file"));
		let report = CapabilityReport::collect_with(context);
		let file = report.capabilities.iter().find(|capability| capability.name == "file").unwrap();
		assert_eq!(file.candidates, ["custom-file"]);
		assert_eq!(file.selected.as_deref(), Some("custom-file"));
		assert_eq!(file.override_env.as_deref(), Some("YAZI_FILE_ONE"));
	}

	#[cfg(unix)]
	#[test]
	fn an_executable_with_a_failing_version_probe_is_still_available() {
		let directory = temp_dir("version");
		let script = directory.join("odd-version");
		fs::write(&script, b"#!/bin/sh\nprintf 'odd version output\\n'\nexit 7\n").unwrap();
		let mut permissions = fs::metadata(&script).unwrap().permissions();
		permissions.set_mode(0o755);
		fs::set_permissions(&script, permissions).unwrap();
		let path = resolve_executable(OsStr::new("odd-version"), Some(directory.as_os_str())).unwrap();
		assert_eq!(version_probe(&path, &["--version"]).as_deref(), Some("odd version output"));
	}

	#[test]
	fn spaces_unicode_and_duplicate_path_entries_are_supported() {
		let directory = temp_dir("space-文件");
		executable(&directory.join("tool"));
		let path = std::env::join_paths([directory.as_path(), directory.as_path()]).unwrap();
		let resolved = resolve_executable(OsStr::new("tool"), Some(&path)).unwrap();
		assert_eq!(resolved, directory.join("tool"));
	}

	#[test]
	fn missing_optional_helpers_are_reported_without_an_error() {
		let report = CapabilityReport::collect_with(ProbeContext::with_path(OsString::new()));
		assert!(
			report.capabilities.iter().any(
				|capability| capability.name == "git" && capability.status == CapabilityStatus::Missing
			)
		);
	}
}
