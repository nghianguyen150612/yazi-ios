use mlua::{Function, Lua};

use super::Utils;

// Byte-count contract for `proc_pidinfo`, which returns a `c_int` byte count
// (0/-1 on failure). Only `ret >= expected` is a complete `proc_taskinfo`;
// both sides stay in the signed C-int domain so there is no signed/unsigned
// mixing. Kept as `i32` (== `c_int` on Apple targets) so the mapping is
// unit-testable on every host; the Apple branch passes the real
// `size_of::<proc_taskinfo>()` as `expected`.
#[cfg(any(test, target_os = "macos", target_os = "ios"))]
fn pidinfo_complete(ret: i32, expected: i32) -> bool { ret >= expected }

impl Utils {
	// Resident-memory lookup through public libproc (`proc_pidinfo`,
	// `PROC_PIDTASKINFO`). libc 0.2.189 declares both for every Apple target in
	// `unix/bsd/apple/mod.rs` (file covers `*-apple-*`, no `target_os` gate on
	// these items), so the symbols are compile-safe for `aarch64-apple-ios`
	// via libSystem. Runtime behavior on a jailbroken device (permissions when
	// inspecting a child, sandbox policy) stays DEVICE-UNVERIFIED. Contract:
	// success with a complete `proc_taskinfo` yields `{ mem_resident = ... }`;
	// failure or a partial (short) result yields `{}` so
	// `ya.proc_info(id).mem_resident` is nil, which `svg.lua` treats as
	// "unmeasured" rather than a fatal error. This is distinct from
	// `Command:memory()`, which enforces a limit via Unix
	// `setrlimit(RLIMIT_AS)` in the child before exec.
	#[cfg(any(target_os = "macos", target_os = "ios"))]
	pub(super) fn proc_info(lua: &Lua) -> mlua::Result<Function> {
		lua.create_function(|lua, pid: usize| {
			// `proc_pidinfo` returns the byte count written, or 0/-1 on
			// failure. Only a full `proc_taskinfo` is a valid measurement:
			// a short write must not be treated as a complete structure.
			// Failure or partial result returns an empty table so
			// `ya.proc_info(id).mem_resident` is nil and `svg.lua`'s
			// `if mem and mem > alloc` guard treats it as unmeasured rather
			// than as a zero-byte resident set. Success maps to
			// `{ mem_resident = <bytes> }`.
			let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
			let expected = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
			let ret = unsafe {
				libc::proc_pidinfo(
					pid as _,
					libc::PROC_PIDTASKINFO,
					0,
					&mut info as *mut _ as *mut _,
					expected,
				)
			};
			if !pidinfo_complete(ret, expected) {
				return lua.create_table();
			}
			lua.create_table_from([("mem_resident", info.pti_resident_size)])
		})
	}

	#[cfg(not(any(target_os = "macos", target_os = "ios")))]
	pub(super) fn proc_info(lua: &Lua) -> mlua::Result<Function> {
		lua.create_function(|lua, ()| lua.create_table())
	}
}

#[cfg(test)]
mod tests {
	use super::pidinfo_complete;

	#[test]
	fn pidinfo_byte_count_contract() {
		let expected = 116;
		assert!(!pidinfo_complete(0, expected));
		assert!(!pidinfo_complete(-1, expected));
		assert!(!pidinfo_complete(expected - 1, expected));
		assert!(pidinfo_complete(expected, expected));
		assert!(pidinfo_complete(expected + 1, expected));
	}
}
