use crate::{
    fn_list,
    launcher::plugin_launcher::api::{ApiContext, PluginModuleDeclaration, SherlockPluginFn},
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct LogModule;
impl PluginModuleDeclaration for LogModule {
    const NAME: &'static str = "log";
    const FUNCTIONS: &'static [super::FnEntry] = fn_list![Info, Error];
    const RESTRICTED: &'static [super::FnEntry] = &[];
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
