use crate::{
    launcher::plugin_launcher::api::{ApiContext, SherlockPluginFn, SherlockPluginModule},
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct LogModule;
impl SherlockPluginModule for LogModule {
    const NAME: &'static str = "log";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        Info::register(lua, table, ctx)?;
        Error::register(lua, table, ctx)?;

        Ok(())
    }

    fn docs() -> Vec<super::LuaApiDoc> {
        vec![Info::docs(), Error::docs()]
    }
}

struct Info;
impl SherlockPluginFn for Info {
    const NAME: &'static str = "info";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("msg", "string")];
    const RETURNS: &'static str = "nil";
    const DOC: &'static str = "Logs an informational message.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(
            table, lua,
            |_lua, (msg: String)| {
                eprintln!("[plugin:info] {msg}");
                Ok(())
            }
        )
    }
}

struct Error;
impl SherlockPluginFn for Error {
    const NAME: &'static str = "error";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("msg", "string")];
    const RETURNS: &'static str = "nil";
    const DOC: &'static str = "Logs an error message.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(
            table, lua,
            |_lua, (msg: String)| {
                eprintln!("[plugin:error] {msg}");
                Ok(())
            }
        )
    }
}
