yazi_macro::mod_pub!(entry evaluator fetcher loader preloader previewer provider);

yazi_macro::mod_flat!(coroutine runner spot);

pub static RUNNER: yazi_shim::cell::RoCell<Runner> = yazi_shim::cell::RoCell::new();

pub fn init(setter: fn(&mlua::Lua) -> mlua::Result<()>) {
	crate::loader::init();
	RUNNER.init(Runner { setter });
}

#[cfg(test)]
pub(crate) fn init_tests() {
	static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
	INIT.get_or_init(|| {
		yazi_shim::init_tests();
		yazi_shared::init_tests();
		yazi_config::init_tests();
		crate::init(|_| Ok(()));
	});
}
