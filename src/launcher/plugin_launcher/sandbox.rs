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
        // One directory per test: tests run in parallel and rewrite these files.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sherlock-sandbox-{}-{n}", std::process::id()));
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

    #[test]
    fn on_click_callbacks_are_indexed_and_invokable() {
        use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        let node: PluginUiNode = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     return sherlock._prepare('t', ui.row {
                         ui.text 'no click',
                         ui.button 'go' :on_click(function(id) clicked = id end),
                     })",
                )
                .unwrap(),
            )
            .unwrap();
        let PluginUiNode::Container { children, .. } = node else {
            panic!("expected container");
        };
        assert!(matches!(
            children[0],
            PluginUiNode::Text { on_click: None, .. }
        ));
        assert!(matches!(
            children[1],
            PluginUiNode::Button {
                on_click: Some(1),
                ..
            }
        ));

        let clicked: String = lua
            .unpack(run(&lua, &env, "sherlock._invoke('t', 1); return clicked").unwrap())
            .unwrap();
        assert_eq!(clicked, "t");

        // A new send for the tile replaces its callbacks.
        let stale: bool = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "clicked = nil
                     sherlock._prepare('t', sherlock.ui.text 'plain')
                     sherlock._invoke('t', 1)
                     return clicked == nil",
                )
                .unwrap(),
            )
            .unwrap();
        assert!(stale);
    }

    #[test]
    fn tile_meta_carries_activate_and_actions() {
        use crate::launcher::plugin_launcher::ui_schema::PluginTileContent;

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        let content: PluginTileContent = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     local node = ui.row { ui.text 'x' :on_click(function() end) }
                         :on_activate(function(id) activated = id end)
                         :action('Copy', function(id) copied = id end, { icon = 'copy', exit = true })
                     return sherlock._prepare('t', node)",
                )
                .unwrap(),
            )
            .unwrap();
        // on_activate registered first (root), then the action, then the child's on_click.
        let on_activate = content.meta.on_activate.expect("on_activate");
        assert_eq!(content.meta.actions.len(), 1);
        let action = &content.meta.actions[0];
        assert_eq!(action.name, "Copy");
        assert_eq!(action.icon.as_deref(), Some("copy"));
        assert!(action.exit);

        let result: String = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    &format!(
                        "sherlock._invoke('t', {on_activate}); sherlock._invoke('t', {}); \
                         return activated .. ',' .. copied",
                        action.run
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(result, "t,t");
    }

    #[test]
    fn rich_nodes_convert_to_schema() {
        use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        let node: PluginUiNode = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     return ui.column {
                         ui.image '~/pic.png',
                         ui.progress(0.5) { color = 'accent' },
                         ui.divider(),
                         ui.row { ui.text 'a', ui.spacer(), ui.text 'b' }
                             :hover { background = 'bg_selected' },
                     }",
                )
                .unwrap(),
            )
            .unwrap();
        let PluginUiNode::Container { children, .. } = node else {
            panic!("expected container");
        };
        assert!(matches!(&children[0], PluginUiNode::Image { src, .. } if src == "~/pic.png"));
        assert!(matches!(children[1], PluginUiNode::Progress { value, .. } if value == 0.5));
        assert!(matches!(children[2], PluginUiNode::Divider { .. }));
        let PluginUiNode::Container {
            style,
            children: row,
            ..
        } = &children[3]
        else {
            panic!("expected row");
        };
        assert!(style.hover.is_some());
        assert!(matches!(row[1], PluginUiNode::Spacer { .. }));
    }

    #[test]
    fn hidden_and_search_convert() {
        use crate::launcher::plugin_launcher::ui_schema::PluginNodeRegistration;

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        let reg: PluginNodeRegistration = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "return { id = 't', search = 'weather forecast',
                              node = sherlock._prepare('t', sherlock.ui.text 'x' :hide()) }",
                )
                .unwrap(),
            )
            .unwrap();
        assert!(reg.node.meta.hidden);
        assert_eq!(reg.search.as_deref(), Some("weather forecast"));
    }

    #[test]
    fn query_rows_are_prepared() {
        use crate::launcher::plugin_launcher::ui_schema::PluginNodeRegistration;

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        // Builder node, plain node table, and explicit row with id/search.
        let rows: Vec<PluginNodeRegistration> = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     return sherlock._prepare_rows {
                         ui.text 'a' :on_activate(function(id) hit = id end),
                         { type = 'text', content = 'b' },
                         { id = 'custom', search = 'cc', node = ui.text 'c' },
                     }",
                )
                .unwrap(),
            )
            .unwrap();
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["result:1", "result:2", "custom"]);
        assert_eq!(rows[2].search.as_deref(), Some("cc"));
        let idx = rows[0].node.meta.on_activate.expect("on_activate");

        let hit: String = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    &format!("sherlock._invoke('result:1', {idx}); return hit"),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(hit, "result:1");

        // Rows that disappear release their callbacks.
        let released: bool = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    &format!(
                        "hit = nil; sherlock._prepare_rows {{}}; \
                         sherlock._invoke('result:1', {idx}); return hit == nil"
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        assert!(released);
    }

    #[test]
    fn tile_items_follow_render_order() {
        use crate::launcher::plugin_launcher::ui_schema::{PluginNav, PluginTileContent};

        let (lua, dir) = setup();
        let env = make_env(&lua, &dir).unwrap();
        run(&lua, &env, "sherlock = { ui = {} }").unwrap();
        load_ui_lib(&lua, &env).unwrap();

        let content: PluginTileContent = lua
            .unpack(
                run(
                    &lua,
                    &env,
                    "local ui = sherlock.ui
                     local f = function() end
                     return sherlock._prepare('t', ui.column {
                         nav = 'vertical',
                         ui.text 'title',
                         ui.row { ui.button 'a' :on_click(f), ui.button 'b' :on_click(f) }
                             :on_click(f),
                         ui.button 'c' :on_click(f) :focus { background = 'accent' },
                     })",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(content.meta.nav, PluginNav::Vertical);
        // Row before its children, then c.
        assert_eq!(content.node.item_callbacks().len(), 4);
    }
}
