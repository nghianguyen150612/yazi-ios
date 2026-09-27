use std::sync::OnceLock;

use yazi_runner::loader::{LOADER, Loader};

// LOADER is a process-global RoCell; initialize exactly once per test process.
// Also init the shim (Uzers cache) because `Loader::ensure` resolves the user
// plugin path through `Xdg::config_dir()` → passwd home.
fn init_loader() {
	static INIT: OnceLock<()> = OnceLock::new();
	INIT.get_or_init(|| {
		yazi_shim::init_tests();
		yazi_runner::init(|_| Ok(()));
	});
}

fn chunk_with_header(header: &str) -> Vec<u8> { header.as_bytes().to_vec() }

#[test]
fn compatible_or_error_accepts_empty_since() {
	let chunk = yazi_runner::loader::Chunk::from(chunk_with_header("--- @since \nreturn {}"));
	Loader::compatible_or_error("probe", &chunk).unwrap();
}

#[test]
fn compatible_or_error_rejects_future_since() {
	let chunk = yazi_runner::loader::Chunk::from(chunk_with_header("--- @since 9999.0.0\nreturn {}"));
	let err = Loader::compatible_or_error("probe", &chunk).unwrap_err();
	assert!(err.to_string().contains("requires at least Yazi"), "unexpected: {err}");
}

#[test]
fn chunk_detects_sync_flags() {
	let chunk = yazi_runner::loader::Chunk::from(chunk_with_header(
		"--- @sync peek\n--- @sync entry\nreturn {}",
	));
	assert!(chunk.sync_peek);
	assert!(chunk.sync_entry);
}

#[test]
fn chunk_ignores_non_header_after_code() {
	let chunk = yazi_runner::loader::Chunk::from(chunk_with_header("return {}\n--- @sync peek"));
	assert!(!chunk.sync_peek);
}

#[tokio::test]
async fn invalid_plugin_name_is_rejected_before_io() {
	init_loader();
	let err = LOADER.ensure("Bad_Name", |_| ()).await.unwrap_err();
	assert!(err.to_string().contains("kebab-case"), "unexpected: {err}");
}

#[tokio::test]
async fn invalid_entry_name_is_rejected_before_io() {
	init_loader();
	let err = LOADER.ensure("dds.Bad_Entry", |_| ()).await.unwrap_err();
	assert!(err.to_string().contains("kebab-case"), "unexpected: {err}");
}

#[tokio::test]
async fn missing_user_plugin_is_an_error_not_a_panic() {
	init_loader();
	// A name that is neither built-in nor present under the test XDG dir.
	let err =
		LOADER.ensure("task016-nonexistent-xyz", |_| ()).await.unwrap_err().to_string();
	assert!(
		err.contains("Failed to load plugin") || err.contains("not found"),
		"unexpected: {err}"
	);
}

#[test]
fn builtin_loader_cache_holds_expected_plugins() {
	init_loader();
	for name in ["dds", "empty", "extract", "trash", "folder", "file"] {
		assert!(LOADER.read().contains_key(name), "built-in `{name}` missing from loader cache");
	}
}

#[test]
fn load_chunk_caches_module_in_package_loaded() {
	init_loader();
	let lua = mlua::Lua::new();
	let chunk = yazi_runner::loader::Chunk::from(b"return { answer = 42 }".to_vec());
	let t: mlua::Table = LOADER.load_chunk(&lua, "task016-probe", &chunk).unwrap();
	assert_eq!(t.get::<i64>("answer").unwrap(), 42);
	// Second load hits `package.loaded` and returns the same table.
	let again: mlua::Table = LOADER.load_chunk(&lua, "task016-probe", &chunk).unwrap();
	assert!(t.equals(&again).unwrap());
}

#[test]
fn load_chunk_propagates_lua_syntax_error() {
	init_loader();
	let lua = mlua::Lua::new();
	let chunk = yazi_runner::loader::Chunk::from(b"this is not lua ((( ".to_vec());
	assert!(LOADER.load_chunk(&lua, "task016-broken", &chunk).is_err());
}

#[test]
fn load_chunk_propagates_lua_runtime_error() {
	init_loader();
	let lua = mlua::Lua::new();
	let chunk = yazi_runner::loader::Chunk::from(b"error('boom')".to_vec());
	let err = LOADER.load_chunk(&lua, "task016-runtime-boom", &chunk).unwrap_err().to_string();
	assert!(err.contains("boom"), "unexpected: {err}");
}

#[test]
fn chunk_with_non_utf8_bytes_does_not_panic() {
	// The loader is byte-oriented; source bytes that fail UTF-8 assumptions
	// must not panic the header analyzer.
	let mut bytes = b"--- @since \nreturn {}".to_vec();
	bytes.extend_from_slice(&[0xff, 0xfe, 0x00]);
	let chunk = yazi_runner::loader::Chunk::from(bytes);
	let _ = chunk.sync_peek;
}
