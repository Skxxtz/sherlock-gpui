use crate::launcher::plugin_launcher::{
    api::{http::HttpModule, json::JsonModule, log::LogModule, time::TimeModule, ui::UiModule},
    runtime::TileUpdate,
};
use mlua::prelude::*;
use tokio::sync::mpsc;

mod http;
mod json;
mod log;
mod time;
mod ui;

/// Shared context passed to every module's `register` call.
/// Add fields here as new shared resources show up (e.g. an HTTP client,
/// a config handle, a cancellation token) instead of widening function signatures.
pub struct ApiContext {
    pub update_tx: mpsc::UnboundedSender<TileUpdate>,
}

/// One Lua API domain. Each module owns its own table and its own functions.
trait SherlockPluginModule {
    /// The name the module is exposed under, e.g. `sherlock.http`
    const NAME: &'static str;
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()>;
}

pub fn setup_global_api(lua: &Lua, update_tx: mpsc::UnboundedSender<TileUpdate>) -> LuaResult<()> {
    let ctx = ApiContext { update_tx };
    let sherlock = lua.create_table()?;

    register::<LogModule>(lua, &sherlock, &ctx)?;
    register::<HttpModule>(lua, &sherlock, &ctx)?;
    register::<JsonModule>(lua, &sherlock, &ctx)?;
    register::<TimeModule>(lua, &sherlock, &ctx)?;
    register::<UiModule>(lua, &sherlock, &ctx)?;

    lua.globals().set("sherlock", sherlock)?;
    Ok(())
}

#[inline(always)]
fn register<M: SherlockPluginModule>(
    lua: &Lua,
    sherlock: &LuaTable,
    ctx: &ApiContext,
) -> LuaResult<()> {
    let table = lua.create_table()?;
    M::register(lua, &table, ctx)?;
    sherlock.set(M::NAME, table)?;
    Ok(())
}

fn lua_err(e: impl std::fmt::Display) -> LuaError {
    LuaError::RuntimeError(e.to_string())
}
