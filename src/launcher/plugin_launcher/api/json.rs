use crate::launcher::plugin_launcher::api::{
    ApiContext, SherlockPluginFn, SherlockPluginModule, lua_err,
};
use mlua::prelude::{Lua, LuaResult, LuaSerdeExt, LuaTable};

pub struct JsonModule;
impl SherlockPluginModule for JsonModule {
    const NAME: &'static str = "json";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        Decode::register(lua, table, ctx)?;
        Ok(())
    }
    fn docs() -> Vec<super::LuaApiDoc> {
        vec![Decode::docs()]
    }
}

pub struct Decode;
impl SherlockPluginFn for Decode {
    const NAME: &'static str = "decode";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("input", "string")];
    const RETURNS: &'static str = "table";
    const DOC: &'static str = "Decodes a given string into a Lua table.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        table.set(
            Self::NAME,
            lua.create_function(|lua, input: String| {
                let value: serde_json::Value = serde_json::from_str(&input).map_err(lua_err)?;
                lua.to_value(&value)
            })?,
        )
    }
}
