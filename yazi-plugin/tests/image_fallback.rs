use std::{cell::RefCell, rc::Rc};

use mlua::{Function, Lua, Table, Value};

// Drives the real preset `image.lua` peek() with stubbed Yazi globals and
// records what reaches `ya.preview_widget(job, value)`: "nil", an error
// string, or "text:<rendered metadata>".
fn run_peek(show_err: Option<&str>, info_ok: bool) -> Vec<String> {
	let lua = Lua::new();
	let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));

	lua.globals().set("Url", lua.create_function(|_, s: String| Ok(s)).unwrap()).unwrap();

	let fs = lua.create_table().unwrap();
	fs.set("cha", lua.create_function(|_, _: Value| Ok(false)).unwrap()).unwrap();
	lua.globals().set("fs", fs).unwrap();

	let preview = lua.create_table().unwrap();
	preview.set("image_delay", 0).unwrap();
	let rt = lua.create_table().unwrap();
	rt.set("preview", preview).unwrap();
	lua.globals().set("rt", rt).unwrap();

	let wrap = lua.create_table().unwrap();
	wrap.set("YES", true).unwrap();
	let text = lua.create_table().unwrap();
	text
		.set(
			"parse",
			lua.create_function(|lua, s: String| {
				let t = lua.create_table()?;
				t.set("_text", s)?;
				t.set(
					"area",
					lua.create_function(|_, (this, _): (Table, Value)| Ok(this))?,
				)?;
				t.set("wrap", lua.create_function(|_, (this, _): (Table, Value)| Ok(this))?)?;
				Ok(t)
			})
			.unwrap(),
		)
		.unwrap();
	let ui = lua.create_table().unwrap();
	ui.set("Text", text).unwrap();
	ui.set("Wrap", wrap).unwrap();
	lua.globals().set("ui", ui).unwrap();

	let show_err = show_err.map(str::to_owned);
	let ya = lua.create_table().unwrap();
	ya.set("sleep", lua.create_function(|_, _: Value| Ok(())).unwrap()).unwrap();
	ya.set("file_cache", lua.create_function(|_, _: Value| Ok(Value::Nil)).unwrap()).unwrap();
	ya.set(
		"image_show",
		lua.create_function(move |lua, _: Value| {
			Ok(match &show_err {
				Some(e) => (Value::Nil, Value::String(lua.create_string(e)?)),
				None => (Value::String(lua.create_string("AREA")?), Value::Nil),
			})
		})
		.unwrap(),
	)
	.unwrap();
	ya.set(
		"image_info",
		lua.create_function(move |lua, _: Value| {
			Ok(if info_ok {
				let info = lua.create_table()?;
				info.set("format", "PNG")?;
				info.set("w", 7)?;
				info.set("h", 5)?;
				info.set("color", "Rgb8")?;
				(Value::Table(info), Value::Nil)
			} else {
				(Value::Nil, Value::String(lua.create_string("decode boom")?))
			})
		})
		.unwrap(),
	)
	.unwrap();
	let seen_ = seen.clone();
	ya.set(
		"preview_widget",
		lua.create_function(move |_, (_job, value): (Table, Value)| {
			let s = match value {
				Value::Nil => "nil".to_owned(),
				Value::String(s) => s.to_str().expect("utf-8 error text").to_string(),
				Value::Table(t) => format!("text:{}", t.get::<String>("_text").unwrap_or_default()),
				other => format!("other:{other:?}"),
			};
			seen_.borrow_mut().push(s);
			Ok(())
		})
		.unwrap(),
	)
	.unwrap();
	lua.globals().set("ya", ya).unwrap();

	let path = concat!(env!("CARGO_MANIFEST_DIR"), "/preset/plugins/image.lua");
	let src = std::fs::read_to_string(path).unwrap();
	let m: Table = lua.load(&src).eval().unwrap();
	let peek: Function = m.get("peek").unwrap();

	let file = lua.create_table().unwrap();
	file.set("path", "/tmp/x.png").unwrap();
	let job = lua.create_table().unwrap();
	job.set("file", file).unwrap();
	job.set("area", "AREA").unwrap();
	peek.call::<()>((m, job)).unwrap();

	seen.borrow().clone()
}

#[test]
fn render_success_clears_preview() {
	assert_eq!(run_peek(None, true), ["nil"]);
}

#[test]
fn render_failure_with_valid_image_shows_metadata() {
	let seen = run_peek(Some("renderer boom"), true);
	assert_eq!(seen.len(), 1);
	let text = seen[0].strip_prefix("text:").expect("metadata text preview");
	for want in ["PNG", "7x5", "Rgb8", "No terminal image renderer available", "renderer boom"] {
		assert!(text.contains(want), "missing {want} in:\n{text}");
	}
}

#[test]
fn malformed_image_keeps_decode_error() {
	// The renderer error must not shadow the real decode diagnostic.
	assert_eq!(run_peek(Some("renderer boom"), false), ["decode boom"]);
}
