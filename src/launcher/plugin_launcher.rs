use crate::{
    app::LauncherEntityGlobal,
    define_inner_functions, ensure_func,
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
    utils::{
        errors::{
            SherlockMessage,
            types::{PluginAction, SherlockErrorType},
        },
        files::{expand_path, home_dir},
    },
};
use gpui::{App, AppContext, AsyncApp, SharedString};
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

fn load_plugin(
    path: Arc<Path>,
    capabilities: PluginCapability,
) -> Result<Arc<PluginHandle>, SherlockMessage> {
    #[cfg(test)]
    return Ok(Arc::new(PluginHandle::default()));

    let runtime = LuaRuntimeHandle::get();
    futures::executor::block_on(runtime.load_plugin(path.clone(), capabilities))
        .map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                e
            )
        })
        .map(Arc::new)
}

impl LauncherProvider for PluginLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let home = home_dir()?;
        let Some(path) = raw
            .args
            .get("path")
            .and_then(|p| p.as_str())
            .map(|s| expand_path(s, &home))
        else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidData,
                format!(
                    "Field `path` missing on `{}`",
                    raw.name.as_deref().unwrap_or("PluginLauncher")
                )
            ));
        };
        let path: Arc<Path> = Arc::from(path);

        let capabilities = raw
            .args
            .get("capabilities")
            .and_then(|p| p.as_array())
            .map(|v| capabilities_from_names(v.iter().filter_map(|v| v.as_str())))
            .unwrap_or(PluginCapability::NONE);

        let handle = load_plugin(path.clone(), capabilities)?;

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
                let entity = cx.new(|_| PluginTileState {
                    data: Some(Box::new(data)),
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
                                        Ok(data) => state.set_data(data.into(), cx),
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

#[cfg(feature = "docs")]
mod docs {
    use super::PluginLauncher;
    use crate::{
        display_name,
        docs::launcher::{
            Example, FieldDoc, InnerFunctionDoc, LauncherDoc, LauncherDocEntry,
            plugin_launcher::plugin_capabilities_section,
        },
        variant_name,
    };
    use indoc::indoc;

    impl LauncherDoc for PluginLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(PluginLauncher),
                variant_name: variant_name!(Plugin),
                description: "The harness for custom plugins. Allow access to specific user plugins.",
                args: &[
                    FieldDoc {
                        name: "path",
                        ty: "Path",
                        required: true,
                        default: None,
                        description: "The location of the plugin `init.lua` file.",
                    },
                    FieldDoc {
                        name: "capabilities",
                        ty: "PluginCapability",
                        required: false,
                        default: Some("PluginCapability::None"),
                        description: "The allowed scopes, the plugin can access.",
                    },
                ],
                inner_functions: &[InnerFunctionDoc {
                    name: "Reload",
                    identifier: "inner.reload",
                    description: "Reload plugin and its environment.",
                    user_facing: true,
                }],
                examples: &[Example {
                    description: "Basic plugin launcher",
                    json: indoc! {
                        r#"{
                        "type": "plugin",
                        "name": "Quote Plugin",
                        "args": {
                            "path": "~/.config/sherlock/plugins/quote.lua",
                            "capabilities": ["http.get", "json.decode"]
                        },
                        "actions": [
                            {
                                "name": "Reload",
                                "icon": "sherlock-devtools",
                                "method": "inner.reload"
                            }
                        ],
                        "home": "OnlyHome",
                        "shortcut": false,
                        "spawn_focus": false,
                        "priority": 1
                    }"#
                    },
                }],
                args_explanations: &[plugin_capabilities_section],
                ..LauncherDocEntry::new()
            }
        }
    }
}
