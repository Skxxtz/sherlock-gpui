use super::{
    api::init_local_api,
    capabilities::PluginCapability,
    registry::PluginRegistry,
    runtime::{LuaJob, PluginHandle},
    sandbox::{load_ui_lib, make_env},
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
                    let result = call_plugin_fn_async::<LuaMultiValue>(
                        &lua, &registry, &handle, "live", tile_id,
                    )
                    .await
                    .map(|_| ());
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

/// Short display name: the file stem, or the directory name for `init.lua`.
fn plugin_name(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str());
    let name = match stem {
        Some("init") => path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str()),
        other => other,
    };
    name.unwrap_or("unknown").to_string()
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

    let name = plugin_name(path);

    let root = path.parent().ok_or(LuaError::RuntimeError(format!(
        "plugin '{}' not loaded",
        name
    )))?;

    let env = make_env(lua, root)?;
    init_local_api(lua, &env, Arc::from(path), capabilities)?;
    load_ui_lib(lua, &env)?;

    lua.load(code)
        .set_name(&name)
        .set_environment(env.clone())
        .exec()?;

    let has = |f: &str| matches!(env.get::<LuaValue>(f), Ok(LuaValue::Function(_)));
    let (has_live, has_refresh) = (has("live"), has("refresh"));

    let env_key = lua.create_registry_value(env)?;

    registry
        .borrow_mut()
        .insert(path, env_key)
        .expect("Tried to set new env where one alredy exists.");

    Ok(PluginHandle {
        id: path.to_path_buf(),
        name,
        has_live,
        has_refresh,
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
    args: impl IntoLuaMulti,
) -> LuaResult<R>
where
    R: FromLuaMulti,
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

#[cfg(test)]
mod tests {
    use super::plugin_name;
    use std::path::Path;

    #[test]
    fn plugin_name_uses_dir_for_init_lua() {
        assert_eq!(plugin_name(Path::new("/p/weather/init.lua")), "weather");
        assert_eq!(plugin_name(Path::new("/p/quote.lua")), "quote");
    }
}
