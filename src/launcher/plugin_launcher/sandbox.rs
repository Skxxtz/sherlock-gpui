use mlua::prelude::*;
use std::path::Path;

const SANDBOX_SRC: &str = include_str!("api/assets/sandbox.lua");
const MAKE_ENV_KEY: &str = "sherlock.sandbox.make_env";

/// Creates the Lua VM used for plugins. The `package` library is left out:
/// plugins get a sandboxed `require` instead.
pub fn new_lua() -> LuaResult<Lua> {
    let lua = Lua::new_with(
        LuaStdLib::TABLE | LuaStdLib::STRING | LuaStdLib::MATH | LuaStdLib::COROUTINE,
        LuaOptions::default(),
    )?;
    install(&lua)?;
    Ok(lua)
}

/// Registers the environment builder. Must run before any plugin is loaded.
pub fn install(lua: &Lua) -> LuaResult<()> {
    let make_env: LuaFunction = lua.load(SANDBOX_SRC).set_name("sandbox").eval()?;
    lua.set_named_registry_value(MAKE_ENV_KEY, make_env)
}

const UI_LIB_SRC: &str = include_str!("api/assets/ui.lua");

/// Loads the `sherlock.ui` builder helpers into a plugin env. Expects
/// `sherlock` to already be set on the env.
pub fn load_ui_lib(lua: &Lua, env: &LuaTable) -> LuaResult<()> {
    lua.load(UI_LIB_SRC)
        .set_name("sherlock.ui")
        .set_environment(env.clone())
        .exec()
}

/// Builds a fresh, isolated environment whose `require` resolves inside `root`.
pub fn make_env(lua: &Lua, root: &Path) -> LuaResult<LuaTable> {
    let make_env: LuaFunction = lua.named_registry_value(MAKE_ENV_KEY)?;
    let root = root
        .to_str()
        .ok_or_else(|| LuaError::RuntimeError("plugin path is not valid UTF-8".into()))?;
    make_env.call(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lua: &Lua, env: &LuaTable, code: &str) -> LuaResult<LuaValue> {
        lua.load(code).set_environment(env.clone()).eval()
    }

    fn setup() -> (Lua, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("sherlock-sandbox-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("helper.lua"), "return { value = 42 }").unwrap();
        std::fs::write(dir.join("sub/init.lua"), "return 'nested'").unwrap();
        (new_lua().unwrap(), dir)
    }

    #[test]
    fn require_resolves_inside_plugin_dir() {
        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        let v: i64 = lua
            .unpack(run(&lua, &env, "return require('helper').value").unwrap())
            .unwrap();
        assert_eq!(v, 42);
        let s: String = lua
            .unpack(run(&lua, &env, "return require('sub')").unwrap())
            .unwrap();
        assert_eq!(s, "nested");
        // Cached per plugin: same table on second require.
        let same: bool = lua
            .unpack(run(&lua, &env, "return require('helper') == require('helper')").unwrap())
            .unwrap();
        assert!(same);
    }

    #[test]
    fn require_rejects_traversal() {
        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        for name in ["..", "..x", "a..b", "/etc/passwd", ".hidden", "x."] {
            assert!(
                run(&lua, &env, &format!("return require({name:?})")).is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn plugins_are_isolated() {
        let (lua, dir) = setup();
        let a = make_env(&lua, &dir).unwrap();
        let b = make_env(&lua, &dir).unwrap();
        run(&lua, &a, "_G.leak = 1; string.rep = nil; shared = 1").unwrap();
        let leaked: bool = lua
            .unpack(
                run(
                    &lua,
                    &b,
                    "return leak ~= nil or shared ~= nil or string.rep == nil",
                )
                .unwrap(),
            )
            .unwrap();
        assert!(!leaked);
    }

    #[test]
    fn dangerous_globals_are_absent() {
        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        let ok: bool = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "return package == nil and dofile == nil and loadfile == nil \
                     and os == nil and io == nil and debug == nil \
                     and getmetatable('') == false",
                )
                .unwrap(),
            )
            .unwrap();
        assert!(ok);
    }

    #[test]
    fn load_is_text_only_and_sandboxed() {
        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        // Chunks from `load` see the plugin env, not the real globals.
        let ok: bool = lua
            .unpack(run(&lua, &env, "x = 7; return load('return x')() == 7").unwrap())
            .unwrap();
        assert!(ok);
        // Binary chunks are rejected.
        let rejected: bool = lua
            .unpack(run(&lua, &env, "return load('\\27Lua') == nil").unwrap())
            .unwrap();
        assert!(rejected);
    }

    #[test]
    fn ui_builder_nodes_convert_to_schema() {
        use crate::launcher::plugin_launcher::ui_schema::{PluginNodeRegistration, PluginUiNode};

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        // Builder node, no explicit :build().
        let node: PluginUiNode = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     return ui.row { gap = 4, ui.icon 'search', ui.text 'hi' }",
                )
                .unwrap(),
            )
            .unwrap();
        let PluginUiNode::Container { children, .. } = node else {
            panic!("expected container");
        };
        assert_eq!(children.len(), 2);

        // Builder nodes mixed into a hand-written registration table.
        let reg: PluginNodeRegistration = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "return { id = 't', node = { type = 'container',
                        children = { sherlock.ui.text 'a', { type = 'text', content = 'b' } } } }",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(reg.id, "t");
    }
}
