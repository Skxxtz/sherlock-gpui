use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    launcher::{
        Launcher, LauncherProvider, LauncherType, LoadContext,
        plugin_launcher::{plugin_tile_state::PluginTileState, runtime::LuaRuntimeHandle},
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::widgets::{RenderableChild, plugin::PluginWidget},
    utils::errors::{
        SherlockMessage,
        types::{PluginAction, SherlockErrorType},
    },
    variant_name,
};
use gpui::{AppContext, AsyncApp};
use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;
use std::{path::Path, sync::Arc};

pub mod api;
pub mod job_handler;
pub mod plugin_tile_state;
pub mod registry;
pub mod runtime;
pub mod subscribers;
pub mod ui;
pub mod ui_schema;

#[derive(Clone, Debug, Deserialize)]
pub struct PluginLauncher {
    pub path: Arc<Path>,
}

impl LauncherProvider for PluginLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<PluginLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::Plugin(launcher),
            Err(_) => LauncherType::Empty,
        }
    }

    fn objects(
        &self,
        launcher: Arc<Launcher>,
        ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let runtime: LuaRuntimeHandle = ctx.lua_runtime.clone();

        // 1. Read + load the plugin source. Loading is comparatively fast
        //    (parsing/executing top-level plugin code), so we still do this
        //    as a blocking-ish round trip through the channel, but it runs
        //    on the lua thread, never on the GPUI thread, and never under a
        //    contended global mutex.
        let code = std::fs::read_to_string(&self.path).map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::Plugin(PluginAction::Load, self.path.display().to_string()),
                format!("failed to read plugin file: {e}")
            )
        })?;

        let handle = futures::executor::block_on(runtime.load_plugin(code, self.path.clone()))
            .map_err(|e| {
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Plugin(PluginAction::Load, self.path.display().to_string()),
                    e
                )
            })?;

        // 2. Create one entity per tile *before* we know its real content,
        //    so the UI can render immediately in a loading state.
        //    We don't know tile count yet either — so the very first
        //    `tiles()` call is the one exception that we eagerly await,
        //    but it runs on the dedicated lua thread, not under a shared
        //    lock, so it doesn't block other plugins or the UI thread
        //    indefinitely. If you want even this to be non-blocking, see
        //    the note below about returning a single "loading" tile
        //    immediately and populating the list asynchronously instead.
        let tiles =
            futures::executor::block_on(runtime.call_tiles(handle.clone())).map_err(|e| {
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Plugin(
                        PluginAction::TileInit,
                        self.path.display().to_string()
                    ),
                    e
                )
            })?;

        let children = tiles
            .into_iter()
            .map(|tile| {
                let entity = cx.new(|_cx| PluginTileState {
                    data: Some(tile.clone()),
                    loading: false,
                    error: None,
                });

                let tile_id = "TODO".to_string();
                let weak = entity.downgrade();

                ctx.subscribers.register(tile_id.clone(), weak.clone());

                let has_live =
                    futures::executor::block_on(ctx.lua_runtime.has_fn(handle.clone(), "live"));
                if has_live {
                    ctx.lua_runtime.spawn_live(handle.clone(), tile_id.clone());
                }

                let has_refresh =
                    futures::executor::block_on(ctx.lua_runtime.has_fn(handle.clone(), "refresh"));
                if has_refresh {
                    cx.spawn({
                        let rt = ctx.lua_runtime.clone();
                        let handle = handle.clone();
                        let tile_id = tile_id.clone();
                        let weak = weak.clone();
                        async move |cx: &mut AsyncApp| {
                            let result = rt.call_refresh(handle, tile_id).await;
                            if let Some(entity) = weak.upgrade() {
                                cx.update(|cx| {
                                    entity.update(cx, |state, cx| match result {
                                        Ok(data) => state.set_data(data, cx),
                                        Err(e) => state.set_error(e.to_string(), cx),
                                    });
                                });
                            }
                        }
                    })
                    .detach();
                };

                RenderableChild::Plugin {
                    launcher: Arc::clone(&launcher),
                    inner: PluginWidget {
                        state: entity,
                        tile_id: tile_id.clone(),
                        subscribers: ctx.subscribers.clone(),
                    },
                }
            })
            .collect();
        Ok(children)
    }
}

impl LauncherDoc for PluginLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(PluginLauncher),
            variant_name: variant_name!(Plugin),
            description: "Launches installed desktop applications",
            args: &[FieldDoc {
                name: "use_keywords",
                ty: "bool",
                required: false,
                default: Some("true"),
                description: "Whether the search should use the keywords defined in the .desktop file.",
            }],
            examples: &[Example {
                description: "Basic app launcher",
                json: indoc! {
                    r#"{
                        "name": "App Launcher",
                        "alias": "app",
                        "type": "apps",
                        "args": {
                            "use_keywords": false
                        },
                        "priority": 4,
                        "home": "Home"
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}
