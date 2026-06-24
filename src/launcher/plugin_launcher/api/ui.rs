use crate::{
    launcher::plugin_launcher::{
        api::{ApiContext, SherlockPluginFn, protocol::PluginDeferFunction},
        ui_schema::PluginUiNode,
    },
    lua_fn,
};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct Update;
impl SherlockPluginFn for Update {
    const NAME: &'static str = "update";
    const PARAMS: &'static [(&'static str, &'static str)] =
        &[("tile_id", "string"), ("node", "table")];
    const RETURNS: &'static str = "nil";
    const DOC: &'static str = "Updates the UI tile identified by `<tile_id>` with the given node.";
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()> {
        let update_tx = ctx.update_tx.clone();
        lua_fn!(
            table, lua,
            |_lua, (tile_id: String, node: PluginUiNode)| {
                let _ = update_tx.send(PluginDeferFunction::Update {
                    tile_id,
                    node: Box::new(node),
                });
                Ok(())
            }
        )
    }
}
