use crate::launcher::plugin_launcher::api::{ApiContext, SherlockPluginModule};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct TimeModule;
impl SherlockPluginModule for TimeModule {
    const NAME: &'static str = "time";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        table.set(
            "sleep_ms",
            lua.create_async_function(|_lua, ms: u64| async move {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                Ok(())
            })?,
        )?;
        Ok(())
    }
}
