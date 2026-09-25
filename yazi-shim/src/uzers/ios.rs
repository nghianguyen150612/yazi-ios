use std::{ffi::OsString, mem::MaybeUninit, ptr};

use super::{
	Uzers,
	lookup::{Attempt, LookupError, copy_name, lookup},
};

impl Uzers {
	pub(crate) fn init() {}

	pub fn uid() -> u32 {
		// SAFETY: `getuid` has no arguments or pointer preconditions.
		unsafe { libc::getuid() }
	}

	pub fn gid() -> u32 {
		// SAFETY: `getgid` has no arguments or pointer preconditions.
		unsafe { libc::getgid() }
	}

	pub fn user_name(uid: Option<u32>) -> Option<OsString> {
		user(uid.unwrap_or_else(Self::uid)).ok().flatten()
	}

	pub fn group_name(gid: Option<u32>) -> Option<OsString> {
		group(gid.unwrap_or_else(Self::gid)).ok().flatten()
	}
}

fn user(uid: u32) -> Result<Option<OsString>, LookupError> {
	lookup(uid, |uid, buffer| {
		let mut passwd = MaybeUninit::<libc::passwd>::uninit();
		let mut result = ptr::null_mut();

		// SAFETY: `passwd` and `result` are valid writable pointers, and `buffer` owns
		// exactly the writable byte range passed to the reentrant libc lookup.
		let status = unsafe {
			libc::getpwuid_r(
				uid,
				passwd.as_mut_ptr(),
				buffer.as_mut_ptr().cast(),
				buffer.len(),
				&mut result,
			)
		};
		if status == libc::ERANGE {
			return Attempt::Retry;
		} else if status != 0 {
			return Attempt::Error(status);
		} else if result.is_null() {
			return Attempt::NotFound;
		} else if !ptr::eq(result, passwd.as_mut_ptr()) {
			return Attempt::Error(libc::EINVAL);
		}

		// SAFETY: a successful lookup initialized `passwd` and returned that same
		// address, as required by the `_r` API contract.
		let passwd = unsafe { passwd.assume_init_ref() };
		match name(passwd.pw_name, buffer) {
			Some(name) => Attempt::Found(name),
			None => Attempt::Error(libc::EINVAL),
		}
	})
}

fn group(gid: u32) -> Result<Option<OsString>, LookupError> {
	lookup(gid, |gid, buffer| {
		let mut group = MaybeUninit::<libc::group>::uninit();
		let mut result = ptr::null_mut();

		// SAFETY: `group` and `result` are valid writable pointers, and `buffer` owns
		// exactly the writable byte range passed to the reentrant libc lookup.
		let status = unsafe {
			libc::getgrgid_r(
				gid,
				group.as_mut_ptr(),
				buffer.as_mut_ptr().cast(),
				buffer.len(),
				&mut result,
			)
		};
		if status == libc::ERANGE {
			return Attempt::Retry;
		} else if status != 0 {
			return Attempt::Error(status);
		} else if result.is_null() {
			return Attempt::NotFound;
		} else if !ptr::eq(result, group.as_mut_ptr()) {
			return Attempt::Error(libc::EINVAL);
		}

		// SAFETY: a successful lookup initialized `group` and returned that same
		// address, as required by the `_r` API contract.
		let group = unsafe { group.assume_init_ref() };
		match name(group.gr_name, buffer) {
			Some(name) => Attempt::Found(name),
			None => Attempt::Error(libc::EINVAL),
		}
	})
}

fn name(name: *const libc::c_char, buffer: &[u8]) -> Option<OsString> {
	if name.is_null() {
		return None;
	}

	let offset = (name as usize).checked_sub(buffer.as_ptr() as usize)?;
	copy_name(buffer, offset)
}
