use super::registry::PluginRegistry;
use super::runtime::{LuaJob, PluginHandle, TileDescriptor};
use mlua::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

pub async fn handle_job(lua: Lua, registry: Arc<RefCell<PluginRegistry>>, job: LuaJob) {
    match job {
        LuaJob::LoadPlugin { code, name, reply } => {
            let result = load_plugin(&lua, &registry, &code, &name);
            let _ = reply.send(result);
        }
        LuaJob::CallTiles { handle, reply } => {
            let result =
                call_plugin_fn_async::<Vec<TileDescriptor>>(&lua, &registry, &handle, "tiles", ())
                    .await;
            let _ = reply.send(result);
        }
        LuaJob::CallRefresh {
            handle,
            tile_id,
            reply,
        } => {
            let result = call_plugin_fn_async::<TileDescriptor>(
                &lua, &registry, &handle, "refresh", tile_id,
            )
            .await;
            let _ = reply.send(result);
        }
        LuaJob::SpawnLive { handle, tile_id } => {
            let lua = lua.clone();
            let registry = Arc::clone(&registry);
            tokio::task::spawn_local(async move {
                let result =
                    call_plugin_fn_unit(&lua, &registry, &handle, "live", tile_id.clone()).await;
                if let Err(e) = result {
                    eprintln!("[plugin:{}] live() exited: {e}", handle.name);
                }
            });
        }
        LuaJob::HasFn {
            handle,
            func_name,
            reply,
        } => {
            let result = (|| -> LuaResult<bool> {
                let reg = registry.borrow();
                let plugin = reg
                    .get(handle.id)
                    .ok_or_else(|| LuaError::RuntimeError("plugin not loaded".into()))?;
                let env: LuaTable = lua.registry_value(&plugin.env_key)?;
                Ok(matches!(
                    env.get::<LuaValue>(func_name.as_str()),
                    Ok(LuaValue::Function(_))
                ))
            })();
            let _ = reply.send(result.unwrap_or(false));
        }
        LuaJob::Unload { handle } => {
            if let Some(plugin) = registry.borrow_mut().remove(handle.id) {
                let _ = lua.remove_registry_value(plugin.env_key);
            }
        }
    }
}

fn load_plugin(
    lua: &Lua,
    registry: &Arc<RefCell<PluginRegistry>>,
    code: &str,
    name: &str,
) -> LuaResult<PluginHandle> {
    let env: LuaTable = lua
        .load(
            r#"
            local env = {}
            setmetatable(env, { __index = _G })
            return env
            "#,
        )
        .eval()?;

    lua.load(code)
        .set_name(name)
        .set_environment(env.clone())
        .exec()?;

    let env_key = lua.create_registry_value(env)?;
    let id = registry.borrow_mut().insert(name.to_string(), env_key);

    Ok(PluginHandle {
        id,
        name: name.to_string(),
    })
}

/// Calls a plugin function as a coroutine and drives it to completion,
/// resuming on every yield. Because `tiles`/`refresh` are invoked this way,
/// any `sherlock.*` async function they call internally (which yields under
/// the hood via mlua's async function support) doesn't block this thread —
/// other spawn_local jobs still get polled while we're waiting.
async fn call_plugin_fn_async<R>(
    lua: &Lua,
    registry: &Arc<RefCell<PluginRegistry>>,
    handle: &PluginHandle,
    func_name: &str,
    args: impl IntoLuaMulti + Clone,
) -> LuaResult<R>
where
    R: FromLua,
{
    let env: LuaTable = {
        let reg = registry.borrow();
        let plugin = reg.get(handle.id).ok_or_else(|| {
            LuaError::RuntimeError(format!("plugin '{}' not loaded", handle.name))
        })?;
        lua.registry_value(&plugin.env_key)?
    };

    let f: LuaFunction = env.get(func_name).map_err(|_| {
        LuaError::RuntimeError(format!(
            "plugin '{}' has no function '{}'",
            handle.name, func_name
        ))
    })?;

    // call_async drives the function (and any coroutine yields it triggers
    // via async functions registered in the API) to completion using the
    // tokio executor on this thread — no manual resume loop required.
    f.call_async::<R>(args).await
}

// job_handler.rs — a variant that doesn't try to convert the return value
async fn call_plugin_fn_unit(
    lua: &Lua,
    registry: &Arc<RefCell<PluginRegistry>>,
    handle: &PluginHandle,
    func_name: &str,
    args: impl IntoLuaMulti,
) -> LuaResult<()> {
    let env: LuaTable = {
        let reg = registry.borrow();
        let plugin = reg.get(handle.id).ok_or_else(|| {
            LuaError::RuntimeError(format!("plugin '{}' not loaded", handle.name))
        })?;
        lua.registry_value(&plugin.env_key)?
    };

    let f: LuaFunction = env.get(func_name).map_err(|_| {
        LuaError::RuntimeError(format!(
            "plugin '{}' has no function '{}'",
            handle.name, func_name
        ))
    })?;

    // Discard whatever Lua returns instead of converting it — call_async
    // still needs *some* return type parameter, so use LuaMultiValue,
    // which accepts any number/shape of returned values.
    f.call_async::<LuaMultiValue>(args).await?;
    Ok(())
}
