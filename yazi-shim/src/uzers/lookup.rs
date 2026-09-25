use std::{
	ffi::{OsStr, OsString},
	os::unix::ffi::OsStrExt,
};

const INITIAL_CAPACITY: usize = 2048;

pub(super) enum Attempt<T> {
	Found(T),
	NotFound,
	Error(i32),
	Retry,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum LookupError {
	Status(i32),
	OutOfMemory,
}

pub(super) fn lookup<T>(
	id: u32,
	mut f: impl FnMut(u32, &mut [u8]) -> Attempt<T>,
) -> Result<Option<T>, LookupError> {
	let mut buffer = Vec::new();
	buffer
		.try_reserve_exact(INITIAL_CAPACITY)
		.map_err(|_| LookupError::OutOfMemory)?;
	buffer.resize(INITIAL_CAPACITY, 0);

	loop {
		match f(id, &mut buffer) {
			Attempt::Found(value) => return Ok(Some(value)),
			Attempt::NotFound => return Ok(None),
			Attempt::Error(status) => return Err(LookupError::Status(status)),
			Attempt::Retry => {}
		}

		let len = buffer.len().checked_mul(2).ok_or(LookupError::OutOfMemory)?;
		buffer
			.try_reserve_exact(len - buffer.len())
			.map_err(|_| LookupError::OutOfMemory)?;
		buffer.resize(len, 0);
	}
}

pub(super) fn copy_name(buffer: &[u8], offset: usize) -> Option<OsString> {
	let bytes = buffer.get(offset..)?;
	let len = bytes.iter().position(|&byte| byte == 0)?;
	Some(OsStr::from_bytes(&bytes[..len]).to_owned())
}

#[cfg(test)]
mod tests {
	use std::{
		ffi::OsString,
		os::unix::ffi::OsStringExt,
	};

	use super::{Attempt, LookupError, copy_name, lookup};

	#[test]
	fn maps_known_ids() {
		let user = lookup(501, |id, buffer| {
			assert!(!buffer.is_empty());
			Attempt::Found((id, OsString::from("alice")))
		});
		assert_eq!(user.unwrap(), Some((501, OsString::from("alice"))));

		let group = lookup(502, |id, buffer| {
			assert!(!buffer.is_empty());
			Attempt::Found((id, OsString::from("staff")))
		});
		assert_eq!(group.unwrap(), Some((502, OsString::from("staff"))));
	}

	#[test]
	fn maps_unknown_ids_to_none() {
		let user = lookup(u32::MAX, |_, _| Attempt::<()>::NotFound);
		assert_eq!(user.unwrap(), None);

		let group = lookup(u32::MAX, |_, _| Attempt::<()>::NotFound);
		assert_eq!(group.unwrap(), None);
	}

	#[test]
	fn preserves_lookup_errors() {
		assert_eq!(lookup(501, |_, _| Attempt::<()>::Error(5)), Err(LookupError::Status(5)));
		assert_eq!(lookup(502, |_, _| Attempt::<()>::Error(12)), Err(LookupError::Status(12)));
	}

	#[test]
	fn copies_lossless_names() {
		let bytes = b"j\xffhn\0ignored";
		assert_eq!(copy_name(bytes, 0), Some(OsString::from_vec(bytes[..4].to_vec())));
		assert_eq!(copy_name(bytes, 5), None);
		assert_eq!(copy_name(b"name", 0), None);
	}

	#[test]
	fn grows_lookup_buffers() {
		let mut attempts = 0;
		let result = lookup(42, |_, buffer| {
			attempts += 1;
			match attempts {
				1 => {
					assert_eq!(buffer.len(), 2048);
					Attempt::Retry
				}
				2 => {
					assert_eq!(buffer.len(), 4096);
					Attempt::Found("ok")
				}
				_ => panic!("unexpected lookup attempt"),
			}
		});

		assert_eq!(result.unwrap(), Some("ok"));
		assert_eq!(attempts, 2);
	}
}
