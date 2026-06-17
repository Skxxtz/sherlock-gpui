use super::lua::get_runtime;
use mlua::prelude::*;
use std::path::Path;

#[allow(unused)]
pub struct PluginSandBox {
    env: LuaRegistryKey,
    pub name: String,
}

#[allow(unused)]
impl PluginSandBox {
    pub fn from_file(path: &Path) -> LuaResult<Self> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let code = std::fs::read_to_string(path).map_err(|e| {
            LuaError::RuntimeError(format!("Failed to read plugin '{}': {e}", path.display()))
        })?;

        Self::from_code(&code, name)
    }

    pub fn from_code(code: &str, name: String) -> LuaResult<Self> {
        let lua = get_runtime().lock().unwrap();

        // create isolate environment that still reads from _G as fallback.
        // Plugins can read globals (sherlock API) but writes to own env.
        let env: LuaTable = lua
            .load(
                r#"
            local env = {}
            setmetatable(env, { __index = _G })
            return env
            "#,
            )
            .eval()?;

        // load plugin code into sandbox env
        lua.load(code)
            .set_name(&name)
            .set_environment(env.clone())
            .exec()?;

        // store the env in the registry so it outlives this call
        let key = lua.create_registry_value(env)?;

        Ok(Self { env: key, name })
    }

    pub fn call<A, R>(&self, func: &str, args: A) -> LuaResult<R>
    where
        A: IntoLuaMulti,
        R: FromLuaMulti,
    {
        let lua = get_runtime().lock().unwrap();
        let env: LuaTable = lua.registry_value(&self.env)?;
        let f: LuaFunction = env.get(func)?;
        f.call::<R>(args)
    }

    pub fn has_fn(&self, func: &str) -> bool {
        let lua = get_runtime().lock().unwrap();
        let Ok(env) = lua.registry_value::<LuaTable>(&self.env) else {
            return false;
        };
        matches!(env.get::<LuaValue>(func), Ok(LuaValue::Function(_)))
    }

    pub fn set<V: IntoLua>(&self, key: &str, value: V) -> LuaResult<()> {
        let lua = get_runtime().lock().unwrap();
        let env: LuaTable = lua.registry_value(&self.env)?;
        env.set(key, value)
    }
}

impl Drop for PluginSandBox {
    fn drop(&mut self) {
        if let Ok(lua) = get_runtime().lock() {
            let _ = lua.remove_registry_value(std::mem::replace(
                &mut self.env,
                lua.create_registry_value(LuaValue::Nil).unwrap(),
            ));
        }
    }
}
