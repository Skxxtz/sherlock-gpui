use crate::{
    launcher::plugin_launcher::api::{ApiContext, SherlockPluginFn, protocol::PluginDeferFunction},
    lua_fn,
    utils::clipboard::get_clipboard,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct Get;
impl SherlockPluginFn for Get {
    const NAME: &'static str = "get";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Fetches the current clipboard entry.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            get_clipboard().unwrap_or_default()
        ))
    }
}

pub struct Set;
impl SherlockPluginFn for Set {
    const NAME: &'static str = "set";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("content", "string")];
    const RETURNS: &'static str = "nil";
    const DOC: &'static str = "Writes a string to the clipboard.";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        let update_tx = ctx.update_tx.clone();
        lua_fn!(
            table, lua,
            |_lua, (content: String)| {
                let _ = update_tx.send(PluginDeferFunction::WriteClipboard(content));
                Ok(())
            }
        )
    }
}
