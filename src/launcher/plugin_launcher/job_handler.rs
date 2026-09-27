use super::{
    api::init_local_api,
    capabilities::PluginCapability,
    registry::PluginRegistry,
    runtime::{LuaJob, PluginHandle},
    ui_schema::{PluginNodeRegistration, PluginUiNode},
};
use mlua::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

pub async fn handle_job(lua: Lua, registry: Rc<RefCell<PluginRegistry>>, job: LuaJob) {
    match job {
        LuaJob::LoadPlugin {
            path,
            reply,
            capabilities,
        } => {
            let result = load_plugin(&lua, &registry, &path, capabilities);
            let _ = reply.send(result);
        }
        LuaJob::CallTiles { handle, reply } => {
            let result = call_plugin_fn_async::<Vec<PluginNodeRegistration>>(
                &lua,
                &registry,
                &handle,
                "tiles",
                (),
            )
            .await;
            let _ = reply.send(result);
        }
        LuaJob::CallInit {
            handle,
            theme,
            reply,
        } => {
            let result = if plugin_has_fn(&lua, &registry, &handle, "init") {
                call_plugin_fn_async::<mlua::Value>(
                    &lua,
                    &registry,
                    &handle,
                    "init",
                    theme.as_ref(),
                )
                .await
                .map(|_| ())
            } else {
                Ok(())
            };
            let _ = reply.send(result);
        }
        LuaJob::CallRefresh {
            handle,
            tile_id,
            reply,
        } => {
            let result = if plugin_has_fn(&lua, &registry, &handle, "refresh") {
                call_plugin_fn_async::<PluginUiNode>(&lua, &registry, &handle, "refresh", tile_id)
                    .await
                    .map(Some)
            } else {
                Ok(None)
            };
            let _ = reply.send(result);
        }
        LuaJob::SpawnLive { handle, tile_id } => {
            let task = {
                let lua = lua.clone();
                let registry = Rc::clone(&registry);
                let handle = Arc::clone(&handle);
                tokio::task::spawn_local(async move {
                    let result =
                        call_plugin_fn_unit(&lua, &registry, &handle, "live", tile_id).await;
                    if let Err(e) = result {
                        eprintln!("[plugin:{}] live() exited: {e}", handle.name);
                    }
                })
            };
            match registry.borrow_mut().get_mut(&handle.id) {
                Some(plugin) => plugin.live_tasks.push(task.abort_handle()),
                None => task.abort(),
            }
        }
        LuaJob::HasFn {
            handle,
            func_name,
            reply,
        } => {
            let _ = reply.send(plugin_has_fn(&lua, &registry, &handle, &func_name));
        }
        LuaJob::StopLive { handle } => {
            if let Some(plugin) = registry.borrow_mut().get_mut(&handle.id) {
                for task in plugin.live_tasks.drain(..) {
                    task.abort();
                }
            }
        }
        LuaJob::Unload { handle } => unload_plugin(&lua, &registry, &handle.id),
    }
}

/// Returns whether the plugin's environment defines `func_name` as a function.
fn plugin_has_fn(
    lua: &Lua,
    registry: &Rc<RefCell<PluginRegistry>>,
    handle: &PluginHandle,
    func_name: &str,
) -> bool {
    let reg = registry.borrow();
    let Some(plugin) = reg.get(&handle.id) else {
        return false;
    };
    let Ok(env) = lua.registry_value::<LuaTable>(&plugin.env_key) else {
        return false;
    };
    matches!(env.get::<LuaValue>(func_name), Ok(LuaValue::Function(_)))
}

fn load_plugin(
    lua: &Lua,
    registry: &Rc<RefCell<PluginRegistry>>,
    path: &Path,
    capabilities: PluginCapability,
) -> LuaResult<PluginHandle> {
    let code = std::fs::read_to_string(path)?;
    if registry.borrow().is_loaded(path) {
        unload_plugin(lua, registry, path);
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let root = path.parent().ok_or(LuaError::RuntimeError(format!(
        "plugin '{}' not loaded",
        name
    )))?;

    let package: LuaTable = lua.globals().get("package")?;
    let prev_path: String = package.get("path")?;
    package.set("path", format!("{}/?.lua;{}", root.display(), prev_path))?;

    let env: LuaTable = lua
        .load(
            r#"
            local env = {}
            setmetatable(env, { __index = _G })
            return env
        "#,
        )
        .eval()?;

    if let Err(e) = init_local_api(lua, &env, Arc::from(path), capabilities) {
        package.set("path", prev_path)?;
        return Err(e);
    };

    let plugin_result = lua
        .load(code)
        .set_name(&name)
        .set_environment(env.clone())
        .exec();

    package.set("path", prev_path)?;
    plugin_result?;

    let env_key = lua.create_registry_value(env)?;

    registry
        .borrow_mut()
        .insert(path, env_key)
        .expect("Tried to set new env where one alredy exists.");

    Ok(PluginHandle {
        id: path.to_path_buf(),
        name,
    })
}

#[inline(always)]
pub fn unload_plugin(lua: &Lua, registry: &Rc<RefCell<PluginRegistry>>, id: &Path) {
    let Some(plugin) = registry.borrow_mut().remove(id) else {
        return;
    };
    for task in plugin.live_tasks {
        task.abort();
    }
    let _ = lua.remove_registry_value(plugin.env_key);
}

/// Calls a plugin function as a coroutine and drives it to completion,
/// resuming on every yield. Because `tiles`/`refresh` are invoked this way,
/// any `sherlock.*` async function they call internally (which yields under
/// the hood via mlua's async function support) doesn't block this thread —
/// other spawn_local jobs still get polled while we're waiting.
async fn call_plugin_fn_async<R>(
    lua: &Lua,
    registry: &Rc<RefCell<PluginRegistry>>,
    handle: &PluginHandle,
    func_name: &str,
    args: impl IntoLuaMulti + Clone,
) -> LuaResult<R>
where
    R: FromLua,
{
    let env: LuaTable = {
        let reg = registry.borrow();
        let plugin = reg.get(&handle.id).ok_or_else(|| {
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
    registry: &Rc<RefCell<PluginRegistry>>,
    handle: &PluginHandle,
    func_name: &str,
    args: impl IntoLuaMulti,
) -> LuaResult<()> {
    let env: LuaTable = {
        let reg = registry.borrow();
        let plugin = reg.get(&handle.id).ok_or_else(|| {
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
