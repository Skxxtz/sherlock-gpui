use crate::launcher::plugin_launcher::api::{ApiContext, SherlockPluginModule, lua_err};
use mlua::prelude::{Lua, LuaResult, LuaSerdeExt, LuaTable};

pub struct JsonModule;
impl SherlockPluginModule for JsonModule {
    const NAME: &'static str = "json";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        table.set(
            "decode",
            lua.create_function(|lua, input: String| {
                let value: serde_json::Value = serde_json::from_str(&input).map_err(lua_err)?;
                lua.to_value(&value)
            })?,
        )?;
        Ok(())
    }
}
