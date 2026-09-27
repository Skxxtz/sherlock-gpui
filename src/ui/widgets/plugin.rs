use std::sync::Arc;

use gpui::{
    AnyElement, App, AsyncApp, Div, Entity, ImageSource, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Styled, StyledText, WeakEntity, div, img,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig,
        plugin_launcher::{
            PluginFunctions,
            plugin_tile_state::PluginTileState,
            runtime::{LuaRuntimeHandle, PluginHandle},
            subscribers::TileSubscribers,
            ui_schema::{PluginTileMeta, PluginUiNode},
        },
        utils::exec_mode::ExecMode,
        variant_type::{InnerFunction, LauncherType},
    },
    loader::{resolve_icon_path, utils::Priority},
    ui::{
        launcher::context_menu::{ContextMenuAction, DynamicFunctionAction},
        traits::RenderableChildImpl,
        utils::selection::Selection,
    },
};

#[derive(Clone)]
pub struct PluginWidget {
    pub state: Entity<PluginTileState>,
    pub plugin_id: Arc<std::path::Path>,
    pub tile_id: String,
    pub subscribers: TileSubscribers,
}

impl<'a> RenderableChildImpl<'a> for PluginWidget {
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        _selection: Selection,
        _query: &str,
        _theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        let state = self.state.read(cx);

        if state.loading {
            return div()
                .px_4()
                .py_2()
                .w_full()
                .flex()
                .gap_5()
                .items_center()
                .child("Loading…")
                .into_any_element();
        }

        if let Some(err) = &state.error {
            return div()
                .px_4()
                .py_2()
                .w_full()
                .flex()
                .gap_5()
                .items_center()
                .child(format!("Plugin error: {err}"))
                .into_any_element();
        }

        let Some(data) = &state.data else {
            return div().child("No Child").into_any_element();
        };

        let ctx = NodeCtx {
            handle: match &launcher.launcher_type {
                LauncherType::Plugin(plg) => Some(plg.handle.clone()),
                _ => None,
            },
            tile_id: self.tile_id.clone(),
        };
        render_node(&data.node, &ctx)
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        // Enter runs the tile's `on_activate`, if it has one.
        self.meta(cx)?.on_activate?;
        Some(ExecMode::Inner {
            func: InnerFunction::Plugin(PluginFunctions::Activate),
            exit: false,
        })
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        None
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        "test"
    }
    #[inline(always)]
    fn actions(
        &self,
        launcher: &Arc<LauncherConfig>,
        cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let configured = launcher.actions.as_ref().or(launcher.add_actions.as_ref());
        let plugin_actions = self.plugin_actions(launcher, cx);
        match (configured, plugin_actions.is_empty()) {
            (None, true) => None,
            (Some(c), true) => Some(c.clone()),
            (c, false) => Some(
                plugin_actions
                    .into_iter()
                    .chain(c.into_iter().flat_map(|c| c.iter().cloned()))
                    .collect(),
            ),
        }
    }
    #[inline(always)]
    fn has_actions(&self, cx: &mut App) -> bool {
        self.meta(cx).is_some_and(|m| !m.actions.is_empty())
    }
    #[inline(always)]
    fn vars(&self, _cx: &mut App) -> Option<&[crate::loader::utils::ExecVariable]> {
        None
    }
    #[inline(always)]
    fn increment_count(&self) {}

    fn update_async<C: gpui::AppContext>(&self, launcher: Arc<LauncherConfig>, cx: &mut C) {
        let LauncherType::Plugin(plg) = launcher.launcher_type.as_ref() else {
            return;
        };
        // Loading placeholders have no tile id and nothing to refresh.
        if self.tile_id.is_empty() {
            return;
        }
        let handle = plg.handle.clone();
        let tile_id = self.tile_id.clone();

        self.state.update(cx, |this, cx| {
            let task = cx.spawn(
                move |weak_self: WeakEntity<PluginTileState>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let rt = LuaRuntimeHandle::get();
                        match rt.call_refresh(handle, tile_id).await {
                            Ok(update) => {
                                let _ = weak_self.update(&mut cx, |this, _cx| {
                                    this.error = None;
                                    this.loading = false;
                                    // No `refresh` function: keep current content.
                                    if let Some(update) = update {
                                        this.data = Some(Box::new(update));
                                    }
                                });
                            }
                            Err(e) => {
                                let _ = weak_self.update(&mut cx, |this, _cx| {
                                    this.error = Some(e.to_string());
                                    this.loading = false;
                                    this.data = None;
                                });
                            }
                        }
                    }
                },
            );

            this.update_task = Some(task);
        });
    }
}

impl PluginWidget {
    /// Tile-level behaviour from the current content, if loaded.
    fn meta<'b>(&self, cx: &'b App) -> Option<&'b PluginTileMeta> {
        self.state.read(cx).data.as_ref().map(|d| &d.meta)
    }

    /// Context-menu entries defined by the plugin for this tile.
    fn plugin_actions(
        &self,
        launcher: &Arc<LauncherConfig>,
        cx: &App,
    ) -> Vec<Arc<ContextMenuAction>> {
        let LauncherType::Plugin(plg) = &launcher.launcher_type else {
            return Vec::new();
        };
        let Some(meta) = self.meta(cx) else {
            return Vec::new();
        };
        meta.actions
            .iter()
            .map(|action| {
                let handle = plg.handle.clone();
                let tile_id = self.tile_id.clone();
                let index = action.run;
                let mut entry = DynamicFunctionAction::new(action.name.clone())
                    .exit(action.exit)
                    .on_exec(move |_| {
                        LuaRuntimeHandle::get().invoke_callback(
                            handle.clone(),
                            tile_id.clone(),
                            index,
                        )
                    });
                if let Some(icon) = &action.icon {
                    entry = entry.icon_name(icon);
                }
                Arc::new(ContextMenuAction::Fn(entry))
            })
            .collect()
    }

    /// Runs the tile's `on_activate` callback.
    pub fn activate(&self, handle: Arc<PluginHandle>, cx: &App) {
        if let Some(index) = self.meta(cx).and_then(|m| m.on_activate) {
            LuaRuntimeHandle::get().invoke_callback(handle, self.tile_id.clone(), index);
        }
    }
}

impl Drop for PluginWidget {
    fn drop(&mut self) {
        self.subscribers
            .unregister(&self.plugin_id, &self.tile_id, &self.state);
    }
}

/// What a rendered node needs to route clicks back to its plugin.
struct NodeCtx {
    handle: Option<Arc<PluginHandle>>,
    tile_id: String,
}

impl NodeCtx {
    /// Makes `el` clickable if the node has an `on_click` callback.
    fn clickable(&self, el: Div, on_click: Option<u32>) -> Div {
        let (Some(index), Some(handle)) = (on_click, self.handle.clone()) else {
            return el;
        };
        let tile_id = self.tile_id.clone();
        el.cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                // Don't also activate the launcher row underneath.
                cx.stop_propagation();
                LuaRuntimeHandle::get().invoke_callback(handle.clone(), tile_id.clone(), index);
            })
    }
}

fn render_node(node: &PluginUiNode, ctx: &NodeCtx) -> AnyElement {
    match node {
        PluginUiNode::Container {
            style,
            children,
            on_click,
        } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            ctx.clickable(el, *on_click)
                .children(children.iter().map(|c| render_node(c, ctx)))
                .into_any_element()
        }
        PluginUiNode::Text {
            content,
            style,
            on_click,
        } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            ctx.clickable(el, *on_click)
                .child(StyledText::new(content.clone()))
                .into_any_element()
        }
        PluginUiNode::Icon {
            name,
            style,
            on_click,
        } => {
            let icon = if let Some(icon) = resolve_icon_path(name) {
                if let Some(mut svg) = icon.svg() {
                    style.apply_to_style_refinement(svg.style());
                    svg.into_any_element()
                } else {
                    let mut el = img(icon.clone());
                    style.apply_to_style_refinement(el.style());
                    el.into_any_element()
                }
            } else {
                let mut el = img(ImageSource::Image(Arc::new(gpui::Image::empty())));
                style.apply_to_style_refinement(el.style());
                el.into_any_element()
            };
            match on_click {
                Some(_) => ctx
                    .clickable(div(), *on_click)
                    .child(icon)
                    .into_any_element(),
                None => icon,
            }
        }
        PluginUiNode::Button {
            label,
            style,
            on_click,
        } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            ctx.clickable(el, *on_click)
                .child(label.clone())
                .into_any_element()
        }
    }
}
