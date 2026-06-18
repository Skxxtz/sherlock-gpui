use crate::launcher::plugin_launcher::api::{ApiContext, SherlockPluginModule, lua_err};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct HttpModule;
impl SherlockPluginModule for HttpModule {
    const NAME: &'static str = "http";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        table.set(
            "get",
            lua.create_async_function(|_lua, url: String| async move {
                let resp = reqwest::get(&url).await.map_err(lua_err)?;
                resp.text().await.map_err(lua_err)
            })?,
        )?;
        Ok(())
    }
}
