use crate::{
    launcher::plugin_launcher::api::{ApiContext, SherlockPluginFn, SherlockPluginModule},
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct TimeModule;
impl SherlockPluginModule for TimeModule {
    const NAME: &'static str = "time";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        Sleep::register(lua, table, ctx)
    }

    fn docs() -> Vec<super::LuaApiDoc> {
        vec![Sleep::docs()]
    }
}

struct Sleep;
impl SherlockPluginFn for Sleep {
    const NAME: &'static str = "sleep_ms";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("ms", "number")];
    const RETURNS: &'static str = "nil";
    const DOC: &'static str = "Sleeps for <ms> milliseconds.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(
            @async
            table, lua,
            |_lua, (ms: u64)| async move {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                Ok(())
            }
        )
    }
}
