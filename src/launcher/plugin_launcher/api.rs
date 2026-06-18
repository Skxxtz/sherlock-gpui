use mlua::prelude::*;
use tokio::sync::mpsc;

use super::ui_schema::PluginUiNode;
use crate::launcher::plugin_launcher::runtime::TileUpdate;

pub fn setup_global_api(lua: &Lua, update_tx: mpsc::UnboundedSender<TileUpdate>) -> LuaResult<()> {
    let sherlock = lua.create_table()?;

    sherlock.set(
        "log",
        lua.create_function(|_, (level, msg): (String, String)| {
            match level.as_str() {
                "error" => eprintln!("[plugin:error] {msg}"),
                _ => eprintln!("[plugin:info] {msg}"),
            }
            Ok(())
        })?,
    )?;

    sherlock.set(
        "http_get",
        lua.create_async_function(|_lua, url: String| async move {
            let resp = reqwest::get(&url)
                .await
                .map_err(|e| LuaError::RuntimeError(format!("http_get failed: {e}")))?;
            let body = resp
                .text()
                .await
                .map_err(|e| LuaError::RuntimeError(format!("http_get body failed: {e}")))?;
            Ok(body)
        })?,
    )?;

    sherlock.set(
        "json_decode",
        lua.create_function(|lua, input: String| {
            let value: serde_json::Value =
                serde_json::from_str(&input).map_err(|e| LuaError::RuntimeError(e.to_string()))?;

            lua.to_value(&value)
        })?,
    )?;

    sherlock.set(
        "sleep_ms",
        lua.create_async_function(|_lua, ms: u64| async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            Ok(())
        })?,
    )?;

    // Captures update_tx by move — this is the one and only place this
    // sender needs to live. Every call from any plugin's live() loop goes
    // through this same clone-on-call.
    sherlock.set(
        "update",
        lua.create_function(move |_lua, (tile_id, node): (String, PluginUiNode)| {
            let _ = update_tx.send((tile_id, node));
            Ok(())
        })?,
    )?;

    lua.globals().set("sherlock", sherlock)?;
    Ok(())
}
