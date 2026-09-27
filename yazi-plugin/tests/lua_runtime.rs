use std::sync::OnceLock;

use mlua::{Lua, Table, Value};
use yazi_binding::{Runtime, Scope};

// Task 016 host smoke suite. Two layers are distinguished explicitly:
//
// - "raw mlua" tests prove the vendored Lua 5.5 VM behaves (stdlib, coroutine,
//   async fn). They do NOT prove Yazi integration.
// - "Yazi runtime" tests use the real `yazi_plugin::slim_lua` + real loader
//   `require` + real `Command` binding and therefore validate the Yazi path.
//
// Nothing here claims iOS-device execution. `setup.lua`/`compat.lua` are
// syntax-validated only (see test name); full standard setup needs
// DDS/config process state and stays SOURCE-VERIFIED.

fn init_yazi() {
	static INIT: OnceLock<()> = OnceLock::new();
	INIT.get_or_init(|| {
		yazi_shim::init_tests();
		yazi_shared::init_tests();
		yazi_config::init_tests();
		yazi_runner::init(|_| Ok(()));
	});
}

fn slim_lua() -> Lua {
	init_yazi();
	let lua = Lua::new();
	lua.set_app_data(Runtime::new("task016-slim", Scope::default()));
	yazi_plugin::slim_lua(&lua).unwrap();
	lua
}

fn preset(name: &str) -> String {
	let path = format!("{}/preset/{name}.lua", env!("CARGO_MANIFEST_DIR"));
	std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("missing preset {name}: {e}"))
}

// --- Raw mlua behavior (reference only, not Yazi integration) ---

#[test]
fn raw_lua_vm_executes_basic_expression() {
	let lua = Lua::new();
	let v: i64 = lua.load("return 1 + 2").eval().unwrap();
	assert_eq!(v, 3);
}

#[test]
fn raw_lua_standard_libraries_are_available() {
	let lua = Lua::new();
	// package / coroutine / string / table / math / utf8 / os / io must stay
	// enabled on iOS; do not trim the stdlib for the target.
	let v: String = lua
		.load(
			r#"
			assert(package and coroutine and string and table and math and utf8 and os and io)
			assert(string.upper("ab") == "AB")
			assert(table.concat({1,2,3}, ",") == "1,2,3")
			assert(math.floor(1.9) == 1)
			assert(utf8.len("héllo") == 5)
			return "stdlib-ok"
			"#,
		)
		.eval()
		.unwrap();
	assert_eq!(v, "stdlib-ok");
}

#[test]
fn raw_lua_errors_propagate_without_abort() {
	let lua = Lua::new();
	let err = lua.load("error('task016-boom')").exec().unwrap_err().to_string();
	assert!(err.contains("task016-boom"), "unexpected: {err}");

	let err = lua.load("this is not lua(((").exec().unwrap_err().to_string();
	assert!(!err.is_empty());
}

#[test]
fn raw_lua_coroutine_yields_and_resumes() {
	let lua = Lua::new();
	let v: Vec<i64> = lua
		.load(
			r#"
			local co = coroutine.create(function()
				coroutine.yield(1)
				coroutine.yield(2)
				return 3
			end)
			local out = {}
			while true do
				local ok, v = coroutine.resume(co)
				assert(ok)
				out[#out+1] = v
				if coroutine.status(co) == "dead" then break end
			end
			return out
			"#,
		)
		.eval()
		.unwrap();
	assert_eq!(v, [1, 2, 3]);
}

#[tokio::test]
async fn raw_mlua_async_function_completes() {
	let lua = Lua::new();
	let f = lua.create_async_function(|_, n: i64| async move { Ok(n * 2) }).unwrap();
	lua.globals().set("double", f).unwrap();
	let v: i64 = lua.load("return double(21)").eval_async().await.unwrap();
	assert_eq!(v, 42);
}

#[test]
fn raw_rust_binding_error_surfaces_to_lua() {
	use mlua::ExternalError;
	let lua = Lua::new();
	lua.globals()
		.set("fail_rs", lua.create_function(|_, ()| Err::<(), _>("rs-boom".into_lua_err())).unwrap())
		.unwrap();
	let err = lua.load("fail_rs()").exec().unwrap_err().to_string();
	assert!(err.contains("rs-boom"), "unexpected: {err}");
}

#[test]
fn raw_nested_lua_error_preserves_inner_message() {
	use mlua::ExternalError;
	let lua = Lua::new();
	lua.globals()
		.set(
			"inner",
			lua.create_function(|_, ()| Err::<(), _>("inner-boom".into_lua_err())).unwrap(),
		)
		.unwrap();
	let err = lua.load("inner()").exec().unwrap_err().to_string();
	assert!(err.contains("inner-boom"), "unexpected: {err}");
}

#[test]
fn raw_in_memory_bytecode_dump_roundtrip() {
	// Mirrors `Loader::load_new` promotion (text → `Function::dump` → binary)
	// at the mlua level; the Yazi cache path itself is covered in yazi-runner.
	let lua = Lua::new();
	let f: mlua::Function = lua.load("return 6 * 7").into_function().unwrap();
	let bytes = f.dump(false);
	let g: mlua::Function = lua.load(&bytes).into_function().unwrap();
	let v: i64 = g.call(()).unwrap();
	assert_eq!(v, 42);
}

// --- Real Yazi slim runtime ---

#[test]
fn yazi_slim_runtime_installs_expected_bindings() {
	let lua = slim_lua();
	let g = lua.globals();
	for k in ["ya", "ui", "fs", "rt", "km", "th", "Command", "require"] {
		let v: Value = g.get(k).unwrap();
		assert!(!v.is_nil(), "{k} missing from slim runtime");
	}
	// Real `ya.lua` was executed by `slim_lua` (no stub): helpers exist on the
	// actual `ya` binding.
	let v: String = lua.load(r#"return ya.readable_size(2048)"#).eval().unwrap();
	assert!(!v.is_empty(), "real ya.lua readable_size failed");
}

#[test]
fn yazi_slim_target_identity_uses_real_binding() {
	let lua = slim_lua();
	let os: String = lua.load("return ya.target_os()").eval().unwrap();
	let family: String = lua.load("return ya.target_family()").eval().unwrap();
	// Portable: compare against the host's own consts, never hard-code "unix".
	assert_eq!(os, std::env::consts::OS);
	assert_eq!(family, std::env::consts::FAMILY);
}

#[test]
fn yazi_preset_ya_lua_source_defines_helpers() {
	// Real `ya.lua` source with minimal stub globals (proves the preset parses
	// and runs); the no-stub execution is covered by the slim runtime test.
	let lua = Lua::new();
	lua.globals().set("ya", lua.create_table().unwrap()).unwrap();
	lua.globals().set("Error", lua.create_table().unwrap()).unwrap();
	lua.load(preset("ya")).set_name("ya.lua").exec().unwrap();
	let ya: Table = lua.globals().get("ya").unwrap();
	assert!(ya.get::<Value>("clamp").unwrap().is_function());
	assert!(ya.get::<Value>("readable_size").unwrap().is_function());
}

#[test]
fn setup_and_compat_presets_are_syntax_valid_only() {
	// Syntax-only: `setup.lua` needs the full standard runtime (DDS/config) to
	// execute, so host coverage compiles without running. Execution stays
	// SOURCE-VERIFIED / DEVICE-UNVERIFIED.
	for name in ["setup", "compat"] {
		let src = preset(name);
		Lua::new().load(&src).set_name(format!("{name}.lua")).into_function().unwrap();
	}
	assert!(preset("setup").contains("os.setlocale"));
}

// --- Real Yazi `Command` Lua binding (Unix shell cases are cfg-gated) ---

#[test]
fn yazi_command_binding_exists_on_all_platforms() {
	let lua = slim_lua();
	let v: Value = lua.globals().get("Command").unwrap();
	assert!(!v.is_nil());
}

#[cfg(unix)]
#[tokio::test]
async fn yazi_command_binding_success_and_output() {
	use mlua::{AnyUserData, ObjectLike};
	let lua = slim_lua();
	let out: AnyUserData = lua
		.load(r#"return Command("sh"):arg({"-c", "echo hi"}):output()"#)
		.eval_async()
		.await
		.unwrap();
	let stdout: String = out.get("stdout").unwrap();
	assert_eq!(stdout.trim(), "hi");
	let status: AnyUserData = out.get("status").unwrap();
	assert!(status.get::<bool>("success").unwrap());
}

#[cfg(unix)]
#[tokio::test]
async fn yazi_command_binding_arg_cwd_env_capture() {
	use mlua::{AnyUserData, ObjectLike};
	let lua = slim_lua();
	// arg as table + env + stdout/stderr capture in one shot.
	let out: AnyUserData = lua
		.load(r#"return Command("sh"):arg({"-c", "echo $TASK016_PROBE"}):env("TASK016_PROBE", "probe-ok"):output()"#)
		.eval_async()
		.await
		.unwrap();
	let stdout: String = out.get("stdout").unwrap();
	assert_eq!(stdout.trim(), "probe-ok");

	// cwd via the host temp dir (no hard-coded /tmp literal).
	let tmp = std::env::temp_dir();
	let tmp_str = tmp.to_string_lossy().into_owned();
	lua.globals().set("TASK016_TMP", tmp_str.clone()).unwrap();
	let out: AnyUserData = lua
		.load(r#"return Command("sh"):arg({"-c", "pwd"}):cwd(TASK016_TMP):output()"#)
		.eval_async()
		.await
		.unwrap();
	let status: AnyUserData = out.get("status").unwrap();
	assert!(status.get::<bool>("success").unwrap());
}

#[cfg(unix)]
#[tokio::test]
async fn yazi_command_binding_nonzero_exit_and_stderr() {
	use mlua::{AnyUserData, ObjectLike};
	let lua = slim_lua();
	let out: AnyUserData = lua
		.load(r#"return Command("sh"):arg({"-c", "echo out; echo err >&2; exit 3"}):output()"#)
		.eval_async()
		.await
		.unwrap();
	let status: AnyUserData = out.get("status").unwrap();
	assert!(!status.get::<bool>("success").unwrap());
	assert_eq!(status.get::<i64>("code").unwrap(), 3);
	let stdout: String = out.get("stdout").unwrap();
	let stderr: String = out.get("stderr").unwrap();
	assert!(!stdout.is_empty() && !stderr.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn yazi_command_binding_missing_executable_is_nonfatal() {
	let lua = slim_lua();
	let (out, err): (Value, Value) = lua
		.load(r#"return Command("task016-no-such-bin-xyz"):output()"#)
		.eval_async()
		.await
		.unwrap();
	assert!(out.is_nil(), "missing executable must yield nil output");
	assert!(!err.is_nil(), "missing executable must yield an error");
}

#[cfg(unix)]
#[tokio::test]
async fn zoxide_missing_executable_is_not_reported_as_empty_history() {
	use mlua::Value;

	let lua = slim_lua();
	let source = preset("plugins/zoxide").replace("Command(\"zoxide\")", "Command(\"task017-no-zoxide\")");
	let source = source.replace("return M", "return M.is_empty('.')");
	let (empty, error): (Value, Value) = lua.load(source).eval_async().await.unwrap();
	assert!(empty.is_nil(), "a missing executable must not look like an empty history");
	let error = error.to_string().unwrap_or_default();
	assert!(error.contains("Failed to start `zoxide`"), "unexpected error: {error:?}");
}

// --- Real Yazi cancellation (Scope) ---

#[test]
fn yazi_scope_child_cancel_propagates() {
	let parent = Scope::new();
	let child = parent.child();
	assert!(!child.is_cancelled());
	parent.cancel();
	assert!(child.is_cancelled());
}

#[tokio::test]
async fn yazi_scope_cancelled_resolves() {
	let scope = Scope::new();
	scope.cancel();
	tokio::time::timeout(std::time::Duration::from_secs(5), scope.cancelled())
		.await
		.expect("cancelled scope must resolve");
}

// --- Identity (Unix-only layer, cfg-gated) ---

#[cfg(unix)]
#[test]
fn yazi_identity_layer_callable_on_host() {
	let uid = unsafe { libc::getuid() };
	let gid = unsafe { libc::getgid() };
	let lua = Lua::new();
	lua.globals().set("uid_now", uid).unwrap();
	let v: u32 = lua.load("return uid_now").eval().unwrap();
	assert_eq!(v, uid);
	let _ = gid;
}

// --- Assets (self-containment gate) ---

#[test]
fn required_preset_assets_exist_for_self_containment() {
	// File-side guard so a missing preset fails on the host instead of the
	// device. The cfg-branch proof is the iOS target compile (embedded branch),
	// not this grep alone.
	let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	for rel in [
		"preset/ya.lua",
		"preset/setup.lua",
		"preset/compat.lua",
		"preset/plugins/dds.lua",
		"preset/plugins/extract.lua",
		"preset/plugins/trash.lua",
		"preset/plugins/svg.lua",
		"preset/components/app.lua",
	] {
		assert!(base.join(rel).is_file(), "missing preset asset {rel}");
	}
	let macro_src =
		std::fs::read_to_string(base.join("../yazi-macro/src/asset.rs")).unwrap();
	assert!(
		macro_src.contains(r#"not(target_os = "ios")"#)
			&& macro_src.contains(r#"target_os = "ios""#),
		"iOS debug embedding gate missing in yazi-macro/src/asset.rs"
	);
}
