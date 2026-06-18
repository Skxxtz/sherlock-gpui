use crate::launcher::plugin_launcher::api::{ApiContext, SherlockPluginModule};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct LogModule;
impl SherlockPluginModule for LogModule {
    const NAME: &'static str = "log";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        table.set(
            "info",
            lua.create_function(|_, msg: String| {
                eprintln!("[plugin:info] {msg}");
                Ok(())
            })?,
        )?;
        table.set(
            "error",
            lua.create_function(|_, msg: String| {
                eprintln!("[plugin:error] {msg}");
                Ok(())
            })?,
        )?;
        Ok(())
    }
}
