use crate::{
    fn_list,
    launcher::plugin_launcher::api::{ApiContext, PluginModuleDeclaration, SherlockPluginFn},
    lua_fn,
    utils::clipboard::get_clipboard,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct ClipboardModule;
impl PluginModuleDeclaration for ClipboardModule {
    const NAME: &'static str = "clipboard";
    const FUNCTIONS: &'static [super::FnEntry] = &[];
    const RESTRICTED: &'static [super::FnEntry] = fn_list![Get];
}

struct Get;
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
