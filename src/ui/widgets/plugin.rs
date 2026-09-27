use std::{cell::Cell, sync::Arc};

use gpui::{
    AnyElement, App, AppContext, AsyncApp, Div, Entity, ImageSource, InteractiveElement,
    IntoElement, MouseButton, ObjectFit, ParentElement, Resource, SharedString, SharedUri, Styled,
    StyledImage, StyledText, WeakEntity, div, img, prelude::FluentBuilder, px, relative,
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
            ui::style::{PluginStyle, parse_color},
            ui_schema::{PluginNav, PluginTileMeta, PluginUiNode},
        },
        utils::exec_mode::ExecMode,
        variant_type::{InnerFunction, LauncherType},
    },
    loader::{resolve_icon_path, utils::Priority},
    ui::{
        launcher::{
            context_menu::{ContextMenuAction, DynamicFunctionAction},
            views::MoveDirection,
        },
        traits::RenderableChildImpl,
        utils::selection::Selection,
    },
    utils::files::{expand_path, home_dir},
};

#[derive(Clone)]
pub struct PluginWidget {
    pub state: Entity<PluginTileState>,
    pub plugin_id: Arc<std::path::Path>,
    pub tile_id: String,
    pub subscribers: TileSubscribers,
    pub search: SharedString,
    pub has_on_query: bool,
}

impl<'a> RenderableChildImpl<'a> for PluginWidget {
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
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
            theme,
            focused: state.focused_item.filter(|_| selection.is_selected),
            item_counter: Cell::new(0),
        };
        render_node(&data.node, &ctx)
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        let has_focus = self.state.read(cx).focused_item.is_some();
        if !has_focus {
            self.meta(cx)?.on_activate?;
        }
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
        &self.search
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, cx: &mut C) -> Option<bool> {
        let hidden = self
            .state
            .read_with(cx, |s, _| s.data.as_ref().is_some_and(|d| d.meta.hidden));
        if hidden {
            Some(false)
        } else if self.has_on_query {
            // Plugins with `on_query` filter themselves.
            Some(true)
        } else {
            None
        }
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
    fn move_inner(&self, direction: &MoveDirection, cx: &mut App) -> bool {
        let Some(data) = self.state.read(cx).data.as_ref() else {
            return false;
        };
        let items = data.node.item_callbacks().len();
        if items == 0 {
            return false;
        }
        let forward = match (data.meta.nav, direction) {
            (PluginNav::Horizontal | PluginNav::Both, MoveDirection::Right)
            | (PluginNav::Vertical | PluginNav::Both, MoveDirection::Down) => true,
            (PluginNav::Horizontal | PluginNav::Both, MoveDirection::Left)
            | (PluginNav::Vertical | PluginNav::Both, MoveDirection::Up) => false,
            _ => return false,
        };
        let current = self.state.read(cx).focused_item;
        let next = match (current, forward) {
            (None, true) => Some(0),
            (Some(i), true) if i + 1 < items => Some(i + 1),
            (Some(_), true) => return false,
            (None, false) => return false,
            (Some(0), false) => None,
            (Some(i), false) => Some(i - 1),
        };
        self.state.update(cx, |state, cx| {
            state.focused_item = next;
            cx.notify();
        });
        true
    }
    fn reset_inner(&self, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            if state.focused_item.take().is_some() {
                cx.notify();
            }
        });
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
        let state = self.state.read(cx);
        let focused = state.focused_item.and_then(|i| {
            let data = state.data.as_ref()?;
            data.node.item_callbacks().get(i).copied()
        });
        if let Some(index) = focused.or_else(|| self.meta(cx).and_then(|m| m.on_activate)) {
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

/// What a rendered node needs: click routing and the active theme.
struct NodeCtx {
    handle: Option<Arc<PluginHandle>>,
    tile_id: String,
    theme: Arc<ThemeData>,
    /// Focused item index; items are counted in render order.
    focused: Option<usize>,
    item_counter: Cell<usize>,
}

impl NodeCtx {
    /// Applies `style` (and its `hover` overrides) to a div.
    fn styled(&self, mut el: Div, style: &PluginStyle) -> Div {
        style.apply_to_style_refinement(el.style(), &self.theme);
        match style.hover.clone() {
            Some(hover) => {
                let theme = self.theme.clone();
                el.hover(move |mut s| {
                    hover.apply_to_style_refinement(&mut s, &theme);
                    s
                })
            }
            None => el,
        }
    }

    /// Makes `el` clickable if the node has an `on_click` callback. Such
    /// nodes are the tile's navigable items; the focused one gets `focus`.
    fn clickable(&self, mut el: Div, on_click: Option<u32>, focus: Option<&PluginStyle>) -> Div {
        let Some(index) = on_click else {
            return el;
        };
        let item = self.item_counter.get();
        self.item_counter.set(item + 1);
        if self.focused == Some(item) {
            match focus {
                Some(focus) => focus.apply_to_style_refinement(el.style(), &self.theme),
                None => el = el.bg(self.theme.bg_selected).rounded_sm(),
            }
        }
        let Some(handle) = self.handle.clone() else {
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

    /// Styled, optionally clickable div.
    fn node_div(&self, style: &PluginStyle, on_click: Option<u32>) -> Div {
        self.clickable(self.styled(div(), style), on_click, style.focus.as_deref())
    }
}

/// Resolves an image `src`: http(s) URLs as-is, otherwise a path (`~` expanded).
fn image_source(src: &str) -> ImageSource {
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageSource::Resource(Resource::Uri(SharedUri::from(src.to_string())));
    }
    let path = match home_dir() {
        Ok(home) => expand_path(src, &home),
        Err(_) => src.into(),
    };
    ImageSource::Resource(Resource::Path(Arc::from(path)))
}

fn render_node(node: &PluginUiNode, ctx: &NodeCtx) -> AnyElement {
    match node {
        PluginUiNode::Container {
            style,
            children,
            on_click,
        } => ctx
            .node_div(style, *on_click)
            .children(children.iter().map(|c| render_node(c, ctx)))
            .into_any_element(),
        PluginUiNode::Text {
            content,
            style,
            on_click,
        } => ctx
            .node_div(style, *on_click)
            .child(StyledText::new(content.clone()))
            .into_any_element(),
        PluginUiNode::Button {
            label,
            style,
            on_click,
        } => ctx
            .node_div(style, *on_click)
            .child(label.clone())
            .into_any_element(),
        PluginUiNode::Icon {
            name,
            style,
            on_click,
        } => {
            let icon = if let Some(icon) = resolve_icon_path(name) {
                if let Some(mut svg) = icon.svg() {
                    style.apply_to_style_refinement(svg.style(), &ctx.theme);
                    svg.into_any_element()
                } else {
                    let mut el = img(icon.clone());
                    style.apply_to_style_refinement(el.style(), &ctx.theme);
                    el.into_any_element()
                }
            } else {
                let mut el = img(ImageSource::Image(Arc::new(gpui::Image::empty())));
                style.apply_to_style_refinement(el.style(), &ctx.theme);
                el.into_any_element()
            };
            match on_click {
                Some(_) => ctx
                    .clickable(div(), *on_click, style.focus.as_deref())
                    .child(icon)
                    .into_any_element(),
                None => icon,
            }
        }
        PluginUiNode::Image {
            src,
            style,
            on_click,
        } => {
            let mut el = img(image_source(src)).object_fit(ObjectFit::Contain);
            style.apply_to_style_refinement(el.style(), &ctx.theme);
            match on_click {
                Some(_) => ctx
                    .clickable(div(), *on_click, style.focus.as_deref())
                    .child(el)
                    .into_any_element(),
                None => el.into_any_element(),
            }
        }
        PluginUiNode::Progress {
            value,
            style,
            on_click,
        } => {
            let fill = style
                .color
                .as_deref()
                .and_then(|c| parse_color(c, &ctx.theme))
                .unwrap_or(ctx.theme.border_selected);
            // Defaults first, so the plugin's style can override them.
            ctx.node_div(style, *on_click)
                .map(|mut el| {
                    let s = el.style();
                    s.size.width.get_or_insert(relative(1.).into());
                    s.size.height.get_or_insert(px(6.).into());
                    s.background
                        .get_or_insert(gpui::Fill::Color(ctx.theme.bg_muted.into()));
                    el
                })
                .rounded_full()
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .w(relative(value.clamp(0., 1.)))
                        .rounded_full()
                        .bg(fill),
                )
                .into_any_element()
        }
        PluginUiNode::Divider { style } => ctx
            .styled(div().w_full().h(px(1.)).bg(ctx.theme.border), style)
            .into_any_element(),
        PluginUiNode::Spacer { style } => ctx.styled(div().flex_grow(), style).into_any_element(),
    }
}
