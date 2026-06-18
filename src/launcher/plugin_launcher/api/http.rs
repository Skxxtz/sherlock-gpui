use crate::{
    launcher::plugin_launcher::api::{ApiContext, SherlockPluginFn, SherlockPluginModule, lua_err},
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct HttpModule;
impl SherlockPluginModule for HttpModule {
    const NAME: &'static str = "http";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        Get::register(lua, table, ctx)?;

        Ok(())
    }
    fn docs() -> Vec<super::LuaApiDoc> {
        vec![Get::docs()]
    }
}

struct Get;
impl SherlockPluginFn for Get {
    const NAME: &'static str = "get";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("url", "string")];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Performs an HTTP GET request and returns the response body.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(
            @async
            table, lua,
            |_lua, (url: String)| async move {
                let resp = reqwest::get(&url).await.map_err(lua_err)?;
                resp.text().await.map_err(lua_err)
            }
        )
    }
}
