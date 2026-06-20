use crate::{
    app::LauncherEntityGlobal,
    define_inner_functions, display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    ensure_func,
    launcher::{
        ExecEffect, LauncherConfig, LauncherId, LauncherProvider, LauncherType, LoadContext,
        plugin_launcher::{
            api::capabilities_from_names,
            capabilities::PluginCapability,
            plugin_tile_state::PluginTileState,
            runtime::{LuaRuntimeHandle, PluginHandle},
            subscribers::{TileSubscribers, TileSubscribersGlobal},
        },
        variant_type::InnerFunction,
    },
    loader::utils::RawLauncher,
    sherlock_msg, skip_func_if_nav,
    ui::{
        launcher::views::MessageViewGlobal,
        widgets::{RenderableChild, plugin::PluginWidget},
    },
    utils::errors::{
        SherlockMessage,
        types::{PluginAction, SherlockErrorType},
    },
    variant_name,
};
use gpui::{App, AppContext, AsyncApp, SharedString};
use indoc::indoc;
use serde_json::Value;
use std::{path::Path, rc::Rc, sync::Arc};

pub mod api;
pub mod capabilities;
pub mod job_handler;
pub mod plugin_tile_state;
pub mod registry;
pub mod runtime;
pub mod subscribers;
pub mod ui;
pub mod ui_schema;

define_inner_functions! {
    pub enum PluginFunctions {
        Reload,
    }
}

#[derive(Clone, Debug)]
pub struct PluginLauncher {
    pub path: Arc<Path>,
    pub capabilities: PluginCapability,
    pub handle: Arc<PluginHandle>,
}

impl LauncherProvider for PluginLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let Some(path) = raw.args.get("path").and_then(|p| p.as_str()) else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidData,
                format!(
                    "Field `path` missing on `{}`",
                    raw.name.as_deref().unwrap_or("PluginLauncher")
                )
            ));
        };
        let path: Arc<Path> = Arc::from(Path::new(path));

        let capabilities = raw
            .args
            .get("capabilities")
            .and_then(|p| p.as_array())
            .map(|v| capabilities_from_names(v.iter().filter_map(|v| v.as_str())))
            .unwrap_or(PluginCapability::NONE);

        let runtime = LuaRuntimeHandle::get();
        let handle = Arc::new(
            futures::executor::block_on(runtime.load_plugin(path.clone(), capabilities)).map_err(
                |e| {
                    sherlock_msg!(
                        Error,
                        SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                        e
                    )
                },
            )?,
        );

        Ok(LauncherType::Plugin(Self {
            path,
            capabilities,
            handle,
        }))
    }

    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        self.reload_objects(launcher, ctx.subscribers.clone(), cx)
    }

    fn execute_function(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
        _variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        skip_func_if_nav!(func);
        let func = ensure_func!(func, InnerFunction::Plugin);

        let RenderableChild::Plugin { launcher, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack plugin tile but received: {:?}", child)
            ));
        };

        match func {
            PluginFunctions::Reload => {
                let path = self.path.clone();
                let caps = self.capabilities;
                let id = launcher.id();
                cx.spawn(move |cx: &mut AsyncApp| {
                    let cx = cx.clone();
                    async move { reload_plugin(id, path, caps, cx).await }
                })
                .detach();
            }
        }
        Ok(ExecEffect::None)
    }
}

impl PluginLauncher {
    pub fn reload_objects(
        &self,
        launcher: Arc<LauncherConfig>,
        subscribers: TileSubscribers,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let lua_runtime = LuaRuntimeHandle::get();
        let tiles = futures::executor::block_on(lua_runtime.call_tiles(self.handle.clone()))
            .map_err(|e| {
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Plugin(
                        PluginAction::TileInit,
                        self.path.display().to_string()
                    ),
                    e
                )
            })?;

        Ok(tiles
            .into_iter()
            .map(|tile| {
                let (tile_id, data) = (tile.id, tile.node);
                let entity = cx.new(|_cx| PluginTileState {
                    data: Some(data),
                    loading: false,
                    error: None,
                });

                let weak = entity.downgrade();

                subscribers.register(tile_id.clone(), weak.clone());

                let has_live =
                    futures::executor::block_on(lua_runtime.has_fn(self.handle.clone(), "live"));
                if has_live {
                    lua_runtime.spawn_live(self.handle.clone(), tile_id.clone());
                }

                let has_refresh =
                    futures::executor::block_on(lua_runtime.has_fn(self.handle.clone(), "refresh"));
                if has_refresh {
                    cx.spawn({
                        let rt = lua_runtime.clone();
                        let handle = self.handle.clone();
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
                        subscribers: subscribers.clone(),
                    },
                }
            })
            .collect())
    }
}
async fn reload_plugin(
    launcher_id: LauncherId,
    path: Arc<Path>,
    caps: PluginCapability,
    mut cx: AsyncApp,
) {
    let send_message = |message: SherlockMessage| {
        cx.update(|cx| {
            cx.global::<MessageViewGlobal>()
                .clone()
                .push_message(message, cx)
        });
    };

    let rt = LuaRuntimeHandle::get();
    let handle = match rt.load_plugin(path.clone(), caps).await {
        Ok(h) => h,
        Err(e) => {
            send_message(sherlock_msg!(
                Warning,
                SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                e
            ));
            return;
        }
    };

    let data_entity = cx.update(|cx| cx.global::<LauncherEntityGlobal>().0.clone());
    let _ = data_entity.update(&mut cx, |data, cx| {
        let data_raw = Rc::make_mut(data);
        let Some(launcher) = data_raw.get_mut(&launcher_id) else {
            return;
        };

        let config = Arc::make_mut(&mut launcher.config);
        if let LauncherType::Plugin(plg) = &mut config.launcher_type {
            plg.handle = Arc::new(handle);
        }

        let subs = cx.global::<TileSubscribersGlobal>().0.clone();
        if let LauncherType::Plugin(plg) = &launcher.config.launcher_type
            && let Ok(children) = plg.reload_objects(launcher.config.clone(), subs, cx)
        {
            launcher.children = children;
        }
    });
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
