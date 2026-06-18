use crate::launcher::plugin_launcher::{
    api::{ApiContext, SherlockPluginModule},
    ui_schema::PluginUiNode,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct UiModule;
impl SherlockPluginModule for UiModule {
    const NAME: &'static str = "ui";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        let update_tx = ctx.update_tx.clone();
        table.set(
            "update",
            lua.create_function(move |_lua, (tile_id, node): (String, PluginUiNode)| {
                let _ = update_tx.send((tile_id, node));
                Ok(())
            })?,
        )?;
        Ok(())
    }
}
