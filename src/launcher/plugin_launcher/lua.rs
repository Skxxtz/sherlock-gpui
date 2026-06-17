use std::sync::{Mutex, OnceLock};

use mlua::prelude::*;

static LUA_RUNTIME: OnceLock<Mutex<Lua>> = OnceLock::new();

pub fn get_runtime() -> &'static Mutex<Lua> {
    LUA_RUNTIME.get_or_init(|| {
        let lua = Lua::new_with(
            LuaStdLib::TABLE | LuaStdLib::STRING | LuaStdLib::MATH | LuaStdLib::COROUTINE,
            LuaOptions::default(),
        )
        .expect("Failed to initialize Lua runtime");

        setup_global_api(&lua).expect("Failed to setup Lua API");

        Mutex::new(lua)
    })
}

fn setup_global_api(lua: &Lua) -> LuaResult<()> {
    let sherlock = lua.create_table()?;
    sherlock.set(
        "log",
        lua.create_function(|_, (level, msg): (String, String)| {
            match level.as_str() {
                "error" => eprintln!("[plugin:error] {msg}"),
                _ => eprintln!("[plugin:info] {msg}"),
            }
            Ok(())
        })?,
    )?;

    lua.globals().set("sherlock", sherlock)?;

    Ok(())
}
