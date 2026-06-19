use crate::{
    fn_list,
    launcher::plugin_launcher::api::{
        ApiContext, PluginModuleDeclaration, SherlockPluginFn, lua_err,
    },
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};
use reqwest::Client;

pub struct HttpModule;
impl PluginModuleDeclaration for HttpModule {
    const NAME: &'static str = "http";
    const FUNCTIONS: &'static [super::FnEntry] = &[];
    const RESTRICTED: &'static [super::FnEntry] = fn_list![Get, Post];
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

struct Post;
impl SherlockPluginFn for Post {
    const NAME: &'static str = "post";
    const PARAMS: &'static [(&'static str, &'static str)] =
        &[("url", "string"), ("body", "string"), ("headers", "table?")];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Performs an HTTP POST request with the given body and optional headers, and returns the response body.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(
            @async
            table, lua,
            |_lua, (url: String, body: String, headers: Option<LuaTable>)| {
                let client = Client::new();
                async move {
                    let mut req = client.post(&url).body(body);

                    if let Some(headers) = headers {
                        for pair in headers.pairs::<String, String>() {
                            let (k, v) = pair.map_err(lua_err)?;
                            req = req.header(&k, v);
                        }
                    }

                    let resp = req.send().await.map_err(lua_err)?;
                    resp.text().await.map_err(lua_err)
                }
            }
        )
    }
}
