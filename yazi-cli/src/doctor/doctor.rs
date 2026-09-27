use anyhow::Result;

use crate::capability::CapabilityReport;

pub(crate) struct Doctor;

impl Doctor {
	pub(crate) fn print(json: bool) -> Result<String> {
		let report = CapabilityReport::collect();
		Self::render(&report, json)
	}

	pub(crate) fn render(report: &CapabilityReport, json: bool) -> Result<String> {
		if json { Ok(serde_json::to_string_pretty(report)?) } else { Ok(report.render()) }
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::OsString;

	use super::*;

	#[test]
	fn human_output_is_plain_text_and_grouped() {
		let report =
			CapabilityReport::collect_with(crate::capability::ProbeContext::with_path(OsString::new()));
		let output = Doctor::render(&report, false).unwrap();
		assert!(output.contains("Core"));
		assert!(output.contains("Search"));
		assert!(output.contains("OPTIONAL MISSING"));
		assert!(!output.contains('\x1b'));
	}

	#[test]
	fn json_output_has_stable_fields_without_environment_dump() {
		let mut context = crate::capability::ProbeContext::with_path(OsString::new());
		context.file_one = Some(OsString::from("custom-file"));
		let report = CapabilityReport::collect_with(context);
		let output = Doctor::render(&report, true).unwrap();
		let value: serde_json::Value = serde_json::from_str(&output).unwrap();
		assert!(value.get("target_os").is_some());
		assert!(value.get("target_family").is_some());
		assert!(value.get("path").is_some());
		assert!(value.get("environment").is_none());
		let capabilities = value.get("capabilities").unwrap().as_array().unwrap();
		let git = capabilities.iter().find(|capability| capability["name"] == "git").unwrap();
		assert_eq!(git["status"], "missing");
		assert!(git.get("candidates").is_some());
		assert!(git.get("selected").is_some());
		assert!(git.get("version").is_some());
		let file = capabilities.iter().find(|capability| capability["name"] == "file").unwrap();
		assert_eq!(file["candidates"], serde_json::json!(["custom-file"]));
		assert_eq!(file["override_env"], "YAZI_FILE_ONE");
		let native_clipboard = capabilities
			.iter()
			.find(|capability| capability["name"] == "ios-native-clipboard")
			.unwrap();
		assert_eq!(native_clipboard["status"], "not_applicable");
	}
}
