use yazi_binding::{Runtime, Scope};

// The frame stack is the contract `require` relies on: every `enter_*` must be
// paired with `leave`, including when the nested load fails. These tests pin
// the balanced behavior source-verified in `require.rs` / `macros.rs`.

#[test]
fn new_frame_reports_name_and_module() {
	let rt = Runtime::new("dds.main", Scope::default());
	assert_eq!(rt.name().unwrap(), "dds.main");
	assert_eq!(rt.module().unwrap(), "dds");
	assert!(!rt.is_blocking());
}

#[test]
fn enter_nested_inherits_scope_and_restores_name() {
	let mut rt = Runtime::new("dds", Scope::default());
	rt.enter_nested("dds.child");
	assert_eq!(rt.name().unwrap(), "dds.child");
	assert_eq!(rt.module().unwrap(), "dds");
	rt.leave().unwrap();
	assert_eq!(rt.name().unwrap(), "dds");
}

#[test]
fn enter_inherited_marks_blocking() {
	let mut rt = Runtime::new("init", Scope::default());
	assert!(!rt.is_blocking());
	rt.enter_inherited("init.async", true);
	assert!(rt.is_blocking());
	rt.leave().unwrap();
	assert!(!rt.is_blocking());
}

#[test]
fn leave_underflow_is_an_error_not_a_panic() {
	let mut rt = Runtime::new("x", Scope::default());
	rt.leave().unwrap();
	assert!(rt.leave().is_err());
	assert!(rt.name().is_err());
}

#[test]
fn manual_frame_pairing_mirrors_require_pattern() {
	// Pairing unit test only: proves enter/leave balance around a fallible
	// step. Real `Require::install` balance is covered by the yazi-runner
	// `real_nested_require_*` tests, not by this simulation.
	let mut rt = Runtime::new("dds", Scope::default());
	rt.enter_nested("dds.missing");
	let load: anyhow::Result<()> = Err(anyhow::anyhow!("Plugin `dds.missing` not found"));
	rt.leave().unwrap();
	assert!(load.is_err());
	assert_eq!(rt.name().unwrap(), "dds");
}

#[test]
fn child_scope_is_independent_but_derived() {
	let rt = Runtime::new("dds", Scope::new());
	let (name, child) = rt.name_child_scope().unwrap();
	assert_eq!(&*name, "dds");
	assert!(!child.is_cancelled());
	child.cancel();
	assert!(child.is_cancelled());
}

#[test]
fn put_block_ignores_init_frame() {
	let lua = mlua::Lua::new();
	let f = lua.create_function(|_, ()| Ok(())).unwrap();
	let mut rt = Runtime::new("init", Scope::default());
	assert_eq!(rt.put_block(&f), None);
	assert_eq!(rt.get_block("init", 0), None);
}

#[test]
fn put_and_get_block_roundtrip() {
	let lua = mlua::Lua::new();
	let f = lua.create_function(|_, ()| Ok(1)).unwrap();
	let mut rt = Runtime::new("my-plugin", Scope::default());
	let idx = rt.put_block(&f).unwrap();
	assert_eq!(idx, 0);
	assert!(rt.get_block("my-plugin", 0).is_some());
	assert!(rt.get_block("my-plugin", 1).is_none());
}

#[test]
fn scope_new_child_cancel() {
	let parent = Scope::new();
	let child = parent.child();
	assert!(!child.is_cancelled());
	assert!(!parent.is_cancelled());
	parent.cancel();
	assert!(child.is_cancelled());
	assert!(parent.is_cancelled());
}

#[tokio::test]
async fn scope_cancelled_future_resolves() {
	let scope = Scope::new();
	scope.cancel();
	tokio::time::timeout(std::time::Duration::from_secs(5), scope.cancelled())
		.await
		.expect("cancelled scope must resolve promptly");
}
