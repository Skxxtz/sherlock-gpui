use crate::{
    app::{LauncherEntityGlobal, theme::ActiveTheme},
    define_inner_functions, ensure_func,
    launcher::{
        ExecEffect, LauncherConfig, LauncherId, LauncherProvider, LauncherType, LoadContext,
        plugin_launcher::{
            api::capabilities_from_names,
            capabilities::PluginCapability,
            plugin_tile_state::PluginTileState,
            runtime::{LuaRuntimeHandle, PluginHandle},
            subscribers::{TileSubscribers, TileSubscribersGlobal},
            ui_schema::PluginNodeRegistration,
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
use mlua::prelude::LuaResult;
use serde_json::Value;
use std::{
    path::Path,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub mod api;
pub mod capabilities;
pub mod job_handler;
pub mod plugin_tile_state;
pub mod registry;
pub mod runtime;
pub mod sandbox;
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
    pub load_gen: Arc<AtomicU64>,
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
        // Canonical path = stable plugin identity (registry, tile subscribers).
        let path = path.canonicalize().map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                e
            )
        })?;
        let path: Arc<Path> = Arc::from(path);

        let capabilities = raw
            .args
            .get("capabilities")
            .and_then(|p| p.as_array())
            .map(|v| capabilities_from_names(v.iter().filter_map(|v| v.as_str())))
            .unwrap_or(PluginCapability::NONE);

        let runtime = LuaRuntimeHandle::get();
        let handle = futures::executor::block_on(runtime.load_plugin(path.clone(), capabilities))
            .map_err(|e| {
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                    e
                )
            })
            .map(Arc::new)?;

        Ok(LauncherType::Plugin(Self {
            path,
            capabilities,
            handle,
            load_gen: Arc::default(),
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
        Ok(vec![self.load_tiles_async(
            launcher,
            ctx.subscribers.clone(),
            cx,
        )])
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
    pub fn load_tiles_async(
        &self,
        launcher: Arc<LauncherConfig>,
        subscribers: TileSubscribers,
        cx: &mut App,
    ) -> RenderableChild {
        let placeholder = cx.new(|_| PluginTileState {
            loading: true,
            ..Default::default()
        });

        // Drop state from any previous load of this plugin.
        let rt = LuaRuntimeHandle::get();
        rt.stop_live(self.handle.clone());
        subscribers.clear_plugin(&self.path);

        let generation = self.load_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let load_gen = self.load_gen.clone();
        let theme = cx.global::<ActiveTheme>().0.clone();
        let handle = self.handle.clone();
        let path = self.path.clone();
        let weak_placeholder = placeholder.downgrade();
        {
            let launcher = Arc::clone(&launcher);
            let subscribers = subscribers.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let result: LuaResult<Vec<PluginNodeRegistration>> = async {
                    rt.call_init(handle.clone(), theme).await?;
                    rt.call_tiles(handle.clone()).await
                }
                .await;

                if load_gen.load(Ordering::SeqCst) != generation {
                    return;
                }

                match result {
                    Ok(tiles) => cx.update(|cx| {
                        if !is_current(&launcher, &handle, cx) {
                            return;
                        }
                        let children =
                            build_tiles(&launcher, &path, &handle, tiles, &subscribers, cx);
                        replace_children(&launcher, &handle, children, cx);
                    }),
                    Err(e) => {
                        let _ = weak_placeholder
                            .update(cx, |state, cx| state.set_error(e.to_string(), cx));
                    }
                }
            })
            .detach();
        }

        RenderableChild::Plugin {
            launcher,
            inner: PluginWidget {
                state: placeholder,
                plugin_id: self.path.clone(),
                tile_id: String::new(),
                subscribers,
            },
        }
    }
}

fn build_tiles(
    launcher: &Arc<LauncherConfig>,
    path: &Arc<Path>,
    handle: &Arc<PluginHandle>,
    tiles: Vec<PluginNodeRegistration>,
    subscribers: &TileSubscribers,
    cx: &mut App,
) -> Vec<RenderableChild> {
    let rt = LuaRuntimeHandle::get();
    tiles
        .into_iter()
        .map(|tile| {
            let entity = cx.new(|_| PluginTileState {
                data: Some(Box::new(tile.node)),
                ..Default::default()
            });
            let weak = entity.downgrade();
            subscribers.register(path.clone(), tile.id.clone(), weak.clone());

            if handle.has_live {
                rt.spawn_live(handle.clone(), tile.id.clone());
            }

            if handle.has_refresh {
                let handle = handle.clone();
                let tile_id = tile.id.clone();
                cx.spawn(async move |cx: &mut AsyncApp| {
                    let result = rt.call_refresh(handle, tile_id).await;
                    let _ = weak.update(cx, |state, cx| match result {
                        Ok(Some(data)) => state.set_data(data.into(), cx),
                        Ok(None) => {}
                        Err(e) => state.set_error(e.to_string(), cx),
                    });
                })
                .detach();
            }

            RenderableChild::Plugin {
                launcher: Arc::clone(launcher),
                inner: PluginWidget {
                    state: entity,
                    plugin_id: path.clone(),
                    tile_id: tile.id,
                    subscribers: subscribers.clone(),
                },
            }
        })
        .collect()
}

/// Whether `handle` is still the plugin handle of this launcher (it isn't
/// after a plugin reload or a config re-read).
fn is_current(launcher: &LauncherConfig, handle: &Arc<PluginHandle>, cx: &App) -> bool {
    let Some(data) = cx.global::<LauncherEntityGlobal>().0.upgrade() else {
        return false;
    };
    data.read(cx).get(&launcher.id()).is_some_and(|entry| {
        matches!(&entry.config.launcher_type, LauncherType::Plugin(p) if Arc::ptr_eq(&p.handle, handle))
    })
}

fn replace_children(
    launcher: &LauncherConfig,
    handle: &Arc<PluginHandle>,
    children: Vec<RenderableChild>,
    cx: &mut App,
) {
    let Some(data) = cx.global::<LauncherEntityGlobal>().0.upgrade() else {
        return;
    };
    let id = launcher.id();
    data.update(cx, |data, cx| {
        let is_current = data.get(&id).is_some_and(|entry| {
            matches!(&entry.config.launcher_type, LauncherType::Plugin(p) if Arc::ptr_eq(&p.handle, handle))
        });
        if !is_current {
            return;
        }
        if let Some(entry) = Rc::make_mut(data).get_mut(&id) {
            entry.children = children;
            cx.notify();
        }
    });
}

async fn reload_plugin(
    launcher_id: LauncherId,
    path: Arc<Path>,
    caps: PluginCapability,
    cx: AsyncApp,
) {
    let rt = LuaRuntimeHandle::get();
    let handle = match rt.load_plugin(path.clone(), caps).await {
        Ok(h) => Arc::new(h),
        Err(e) => {
            cx.update(|cx| {
                cx.global::<MessageViewGlobal>().clone().push_message(
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                        e
                    ),
                    cx,
                )
            });
            return;
        }
    };

    cx.update(|cx| {
        let Some(data) = cx.global::<LauncherEntityGlobal>().0.upgrade() else {
            return;
        };
        data.update(cx, |data, cx| {
            let Some(entry) = Rc::make_mut(data).get_mut(&launcher_id) else {
                return;
            };
            let config = Arc::make_mut(&mut entry.config);
            let LauncherType::Plugin(plg) = &mut config.launcher_type else {
                return;
            };
            plg.handle = handle;
            let plg = plg.clone();

            let subs = cx.global::<TileSubscribersGlobal>().0.clone();
            entry.children = vec![plg.load_tiles_async(entry.config.clone(), subs, cx)];
            cx.notify();
        });
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
