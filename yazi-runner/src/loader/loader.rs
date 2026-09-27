use std::{borrow::Cow, ops::Deref};

use anyhow::{Context, Result, bail, ensure};
use hashbrown::HashMap;
use mlua::{ExternalError, Lua, Table, chunk::ChunkMode};
use parking_lot::RwLock;
use yazi_fs::{Xdg, engine::local::Local};
use yazi_macro::plugin_preset as preset;
use yazi_shared::BytesExt;
use yazi_shim::{cell::RoCell, log::LOG_LEVEL};

use super::Chunk;

pub static LOADER: RoCell<Loader> = RoCell::new();

pub struct Loader {
	cache: RwLock<HashMap<String, Chunk>>,
}

impl Deref for Loader {
	type Target = RwLock<HashMap<String, Chunk>>;

	fn deref(&self) -> &Self::Target { &self.cache }
}

impl Default for Loader {
	fn default() -> Self {
		let cache = HashMap::from_iter([
			// Plugins
			("archive".to_owned(), preset!("plugins/archive").into()),
			("clipboard".to_owned(), preset!("plugins/clipboard").into()),
			("code".to_owned(), preset!("plugins/code").into()),
			("dds".to_owned(), preset!("plugins/dds").into()),
			("dnd".to_owned(), preset!("plugins/dnd").into()),
			("empty".to_owned(), preset!("plugins/empty").into()),
			("extract".to_owned(), preset!("plugins/extract").into()),
			("fd".to_owned(), preset!("plugins/fd").into()),
			("file".to_owned(), preset!("plugins/file").into()),
			("folder".to_owned(), preset!("plugins/folder").into()),
			("font".to_owned(), preset!("plugins/font").into()),
			("fzf".to_owned(), preset!("plugins/fzf").into()),
			("image".to_owned(), preset!("plugins/image").into()),
			("init".to_owned(), preset!("plugins/init").into()),
			("json".to_owned(), preset!("plugins/json").into()),
			("magick".to_owned(), preset!("plugins/magick").into()),
			("mime".to_owned(), preset!("plugins/mime").into()),
			("mime.dir".to_owned(), preset!("plugins/mime-dir").into()),
			("mime.local".to_owned(), preset!("plugins/mime-local").into()),
			("mime.remote".to_owned(), preset!("plugins/mime-remote").into()),
			("mime.trash".to_owned(), preset!("plugins/mime-trash").into()),
			("multi".to_owned(), preset!("plugins/multi").into()),
			("noop".to_owned(), preset!("plugins/noop").into()),
			("null".to_owned(), preset!("plugins/null").into()),
			("pdf".to_owned(), preset!("plugins/pdf").into()),
			("rg".to_owned(), preset!("plugins/rg").into()),
			("search".to_owned(), preset!("plugins/search").into()),
			("session".to_owned(), preset!("plugins/session").into()),
			("svg".to_owned(), preset!("plugins/svg").into()),
			("trash".to_owned(), preset!("plugins/trash").into()),
			("vfs".to_owned(), preset!("plugins/vfs").into()),
			("video".to_owned(), preset!("plugins/video").into()),
			("zoxide".to_owned(), preset!("plugins/zoxide").into()),
			// Components
			("app".to_owned(), [][..].into()),
			("backdrop".to_owned(), [][..].into()),
			("current".to_owned(), [][..].into()),
			("entity".to_owned(), [][..].into()),
			("header".to_owned(), [][..].into()),
			("linemode".to_owned(), [][..].into()),
			("marker".to_owned(), [][..].into()),
			("markers".to_owned(), [][..].into()),
			("modal".to_owned(), [][..].into()),
			("parent".to_owned(), [][..].into()),
			("preview".to_owned(), [][..].into()),
			("progress".to_owned(), [][..].into()),
			("rail".to_owned(), [][..].into()),
			("rails".to_owned(), [][..].into()),
			("root".to_owned(), [][..].into()),
			("status".to_owned(), [][..].into()),
			("tab".to_owned(), [][..].into()),
			("tabs".to_owned(), [][..].into()),
			("tasks".to_owned(), [][..].into()),
			("tip".to_owned(), [][..].into()),
			// Reserved
			("history".to_owned(), [][..].into()),
			("inline".to_owned(), [][..].into()),
			("sftp".to_owned(), [][..].into()),
		]);
		Self { cache: RwLock::new(cache) }
	}
}

impl Loader {
	pub async fn ensure<F, T>(&self, name: &str, f: F) -> Result<T>
	where
		F: FnOnce(&Chunk) -> T,
	{
		self.ensure_in(name, Xdg::config_dir(), f).await
	}

	// Test seam: production resolves user plugins under `Xdg::config_dir()`;
	// tests supply an isolated root. No iOS-only search path is introduced.
	pub(crate) async fn ensure_in<F, T>(
		&self,
		name: &str,
		root: &std::path::Path,
		f: F,
	) -> Result<T>
	where
		F: FnOnce(&Chunk) -> T,
	{
		let (full, plugin, entry) = Self::explode_name_parts(name)?;
		if let Some(c) = self.cache.read().get(full) {
			return Self::compatible_or_error(full, c).map(|_| f(c));
		}

		let p = root.join(format!("plugins/{plugin}.yazi/{entry}.lua"));
		let chunk = Local::regular(&p)
			.read()
			.await
			.with_context(|| format!("Failed to load plugin from {p:?}"))?
			.into();

		let result = Self::compatible_or_error(full, &chunk);
		let inspect = f(&chunk);

		self.cache.write().insert(full.to_owned(), chunk);
		result.map(|_| inspect)
	}

	pub(crate) async fn load(&self, lua: &Lua, name: &str) -> mlua::Result<Table> {
		let (name, ..) = Self::explode_name_parts(name)?;

		let loaded: Table = lua.globals().raw_get::<Table>("package")?.raw_get("loaded")?;
		if let Ok(t) = loaded.raw_get(name) {
			return Ok(t);
		}

		let t = self.load_new(lua, name).await?;
		t.raw_set("_id", lua.create_string(name)?)?;

		loaded.raw_set(name, t.clone())?;
		Ok(t)
	}

	async fn load_new(&self, lua: &Lua, name: &str) -> mlua::Result<Table> {
		let (name, ..) = Self::explode_name_parts(name)?;

		let mut mode = ChunkMode::Text;
		let f = match self.cache.read().get(name) {
			Some(c) => {
				mode = c.mode;
				lua.load(c).set_name(name).into_function()
			}
			None => Err(format!("Plugin `{name}` not found").into_lua_err()),
		}?;

		if mode != ChunkMode::Binary {
			let b = f.dump(LOG_LEVEL.get().is_none());
			if let Some(c) = self.cache.write().get_mut(name) {
				c.mode = ChunkMode::Binary;
				c.bytes = Cow::Owned(b);
			}
		}

		f.call_async(()).await
	}

	pub fn load_chunk(&self, lua: &Lua, name: &str, chunk: &Chunk) -> mlua::Result<Table> {
		let (name, ..) = Self::explode_name_parts(name)?;

		let loaded: Table = lua.globals().raw_get::<Table>("package")?.raw_get("loaded")?;
		if let Ok(t) = loaded.raw_get(name) {
			return Ok(t);
		}

		let t: Table = lua.load(chunk).set_name(name).call(())?;
		t.raw_set("_id", lua.create_string(name)?)?;

		loaded.raw_set(name, t.clone())?;
		Ok(t)
	}

	pub fn try_load(&self, lua: &Lua, name: &str) -> mlua::Result<Table> {
		let (name, ..) = Self::explode_name_parts(name)?;
		lua.globals().raw_get::<Table>("package")?.raw_get::<Table>("loaded")?.raw_get(name)
	}

	pub fn compatible_or_error(name: &str, chunk: &Chunk) -> Result<()> {
		if chunk.compatible() {
			return Ok(());
		}

		bail!(
			"Plugin `{name}` requires at least Yazi {}, but your current version is Yazi {}.",
			chunk.since,
			yazi_version::version_long()
		);
	}

	fn explode_name_parts(name: &str) -> anyhow::Result<(&str, &str, &str)> {
		let name = name.strip_suffix(".main").unwrap_or(name);
		let (plugin, entry) =
			if let Some((a, b)) = name.split_once(".") { (a, b) } else { (name, "main") };

		ensure!(plugin.as_bytes().kebab_cased(), "Plugin name `{plugin}` must be in kebab-case");
		ensure!(entry.as_bytes().kebab_cased(), "Entry name `{entry}` must be in kebab-case");
		Ok((name, plugin, entry))
	}
}

#[cfg(test)]
mod tests {
	use mlua::{Lua, Table};
	use yazi_binding::{Runtime, Scope};

	use super::{Chunk, LOADER};

	fn init() { crate::init_tests(); }

	fn lua_with_runtime(name: &str) -> Lua {
		let lua = Lua::new();
		lua.set_app_data(Runtime::new(name, Scope::default()));
		crate::loader::install(&lua).unwrap();
		lua
	}

	fn seed(name: &str, src: &[u8]) {
		LOADER.write().insert(name.to_owned(), Chunk::from(src.to_vec()));
	}

	#[tokio::test]
	async fn real_require_loads_builtin_and_caches() {
		init();
		let lua = lua_with_runtime("task016-req-basic");
		let t: Table =
			lua.load(r#"return require("empty")"#).eval_async().await.unwrap();
		// `require` returns a per-call wrapper; the underlying module is cached
		// in `package.loaded` and carries the `_id`.
		let inner: Table = t.get("__mod").unwrap();
		assert_eq!(inner.get::<String>("_id").unwrap(), "empty");
		let again: Table =
			lua.load(r#"return require("empty")"#).eval_async().await.unwrap();
		let inner_again: Table = again.get("__mod").unwrap();
		assert!(inner.equals(&inner_again).unwrap());
	}

	#[tokio::test]
	async fn real_nested_require_success() {
		init();
		seed("task016-nest-child", b"return { v = 41 }");
		seed("task016-nest-parent", b"local c = require(\"task016-nest-child\"); return { v = c.v + 1 }");
		let lua = lua_with_runtime("task016-nest-parent");
		let t: Table =
			lua.load(r#"return require("task016-nest-parent")"#).eval_async().await.unwrap();
		assert_eq!(t.get::<i64>("v").unwrap(), 42);
	}

	#[tokio::test]
	async fn real_relative_require_success() {
		init();
		seed("task016-relmod.sub", b"return { v = 7 }");
		seed("task016-relmod", b"local s = require(\".sub\"); return { v = s.v * 6 }");
		let lua = lua_with_runtime("task016-relmod");
		let t: Table =
			lua.load(r#"return require("task016-relmod")"#).eval_async().await.unwrap();
		assert_eq!(t.get::<i64>("v").unwrap(), 42);
	}

	#[tokio::test]
	async fn real_nested_require_failure_restores_frame() {
		init();
		seed("task016-fail-parent", b"local _ = require(\"task016-no-such-child-xyz\"); return {}");
		let lua = lua_with_runtime("task016-fail-caller");
		let err = lua
			.load(r#"return require("task016-fail-parent")"#)
			.eval_async::<Table>()
			.await
			.unwrap_err()
			.to_string();
		assert!(
			err.contains("task016-no-such-child-xyz") || err.contains("not found"),
			"unexpected: {err}"
		);
		// The caller frame must be restored after the nested failure.
		let rt = lua.app_data_ref::<Runtime>().unwrap();
		assert_eq!(rt.name().unwrap(), "task016-fail-caller");
	}

	#[tokio::test]
	async fn real_require_missing_module_error_context() {
		init();
		let lua = lua_with_runtime("task016-missing-caller");
		let err = lua
			.load(r#"return require("task016-no-such-xyz")"#)
			.eval_async::<Table>()
			.await
			.unwrap_err()
			.to_string();
		assert!(err.contains("task016-no-such-xyz"), "unexpected: {err}");
	}

	fn unique_root(tag: &str) -> std::path::PathBuf {
		let mut p = std::env::temp_dir();
		p.push(format!("yazi-task016-{}-{}", std::process::id(), tag));
		let _ = std::fs::remove_dir_all(&p);
		std::fs::create_dir_all(&p).unwrap();
		p
	}

	fn write_plugin(root: &std::path::Path, plugin: &str, entry: &str, src: &[u8]) {
		let dir = root.join(format!("plugins/{plugin}.yazi"));
		std::fs::create_dir_all(&dir).unwrap();
		std::fs::write(dir.join(format!("{entry}.lua")), src).unwrap();
	}

	#[tokio::test]
	async fn user_plugin_main_lua_loads_from_config_root() {
		init();
		let root = unique_root("normal");
		write_plugin(&root, "task016-u-normal", "main", b"return { ok = true }");
		let chunk = LOADER
			.ensure_in("task016-u-normal", &root, |c| (c.sync_peek, c.sync_entry))
			.await
			.unwrap();
		assert_eq!(chunk, (false, false));
		let _ = std::fs::remove_dir_all(&root);
	}

	#[tokio::test]
	async fn user_plugin_root_with_spaces_loads() {
		init();
		let mut root = std::env::temp_dir();
		root.push(format!("yazi task016 spaces {}", std::process::id()));
		let _ = std::fs::remove_dir_all(&root);
		std::fs::create_dir_all(&root).unwrap();
		// Plugin NAME stays kebab-case; the surrounding root carries the space.
		write_plugin(&root, "task016-u-spaced", "main", b"return { ok = true }");
		LOADER.ensure_in("task016-u-spaced", &root, |_| ()).await.unwrap();
		let _ = std::fs::remove_dir_all(&root);
	}

	#[tokio::test]
	async fn user_plugin_root_with_unicode_loads() {
		init();
		let mut root = std::env::temp_dir();
		root.push(format!("yazi-task016-é-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&root);
		std::fs::create_dir_all(&root).unwrap();
		write_plugin(&root, "task016-u-unicode", "main", b"return { ok = true }");
		LOADER.ensure_in("task016-u-unicode", &root, |_| ()).await.unwrap();
		let _ = std::fs::remove_dir_all(&root);
	}

	#[tokio::test]
	async fn user_plugin_nested_entry_loads() {
		init();
		let root = unique_root("nested");
		write_plugin(&root, "task016-u-nested", "entry", b"return { v = 1 }");
		LOADER.ensure_in("task016-u-nested.entry", &root, |_| ()).await.unwrap();
		let _ = std::fs::remove_dir_all(&root);
	}

	#[tokio::test]
	async fn user_plugin_missing_main_is_error() {
		init();
		let root = unique_root("missing");
		let err = LOADER
			.ensure_in("task016-u-absent", &root, |_| ())
			.await
			.unwrap_err()
			.to_string();
		assert!(err.contains("Failed to load plugin"), "unexpected: {err}");
		let _ = std::fs::remove_dir_all(&root);
	}

	#[tokio::test]
	async fn user_plugin_syntax_error_after_read() {
		init();
		let root = unique_root("syntax");
		write_plugin(&root, "task016-u-broken", "main", b"this is not lua ((( ");
		LOADER.ensure_in("task016-u-broken", &root, |_| ()).await.unwrap();
		// The bytes load fine; the syntax error surfaces at execution.
		let lua = lua_with_runtime("task016-u-broken");
		let err = lua
			.load(r#"return require("task016-u-broken")"#)
			.eval_async::<Table>()
			.await
			.unwrap_err()
			.to_string();
		assert!(!err.is_empty());
		let _ = std::fs::remove_dir_all(&root);
	}
}
