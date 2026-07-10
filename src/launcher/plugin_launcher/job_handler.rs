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
            let result = call_plugin_fn_async::<mlua::Value>(
                &lua,
                &registry,
                &handle,
                "init",
                theme.as_ref(),
            )
            .await
            .map(|_| ());
            let _ = reply.send(result);
        }
        LuaJob::CallRefresh {
            handle,
            tile_id,
            reply,
        } => {
            let result =
                call_plugin_fn_async::<PluginUiNode>(&lua, &registry, &handle, "refresh", tile_id)
                    .await;
            let _ = reply.send(result);
        }
        LuaJob::SpawnLive { handle, tile_id } => {
            let lua = lua.clone();
            let registry = Rc::clone(&registry);
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
                    .get(&handle.id)
                    .ok_or_else(|| LuaError::RuntimeError("plugin not loaded".into()))?;
                let env: LuaTable = lua.registry_value(&plugin.env_key)?;
                Ok(matches!(
                    env.get::<LuaValue>(func_name.as_str()),
                    Ok(LuaValue::Function(_))
                ))
            })();
            let _ = reply.send(result.unwrap_or(false));
        }
        LuaJob::Unload { handle } => unload_plugin(&lua, &registry, &handle.id),
    }
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

    if let Err(e) = init_local_api(lua, &env, capabilities) {
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
    if let Some(plugin) = registry.borrow_mut().remove(id) {
        let _ = lua.remove_registry_value(plugin.env_key);
    }
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
