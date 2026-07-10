use std::sync::Arc;

use gpui::{
    AnyElement, App, AsyncApp, Entity, ImageSource, IntoElement, ParentElement, Styled, StyledText,
    WeakEntity, div, img,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig,
        plugin_launcher::{
            plugin_tile_state::PluginTileState, runtime::LuaRuntimeHandle,
            subscribers::TileSubscribers, ui_schema::PluginUiNode,
        },
        utils::exec_mode::ExecMode,
        variant_type::LauncherType,
    },
    loader::{resolve_icon_path, utils::Priority},
    ui::{
        launcher::context_menu::ContextMenuAction, traits::RenderableChildImpl,
        utils::selection::Selection,
    },
};

#[derive(Clone)]
pub struct PluginWidget {
    pub state: Entity<PluginTileState>,
    pub tile_id: String,
    pub subscribers: TileSubscribers,
}

impl<'a> RenderableChildImpl<'a> for PluginWidget {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
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

        render_node(data)
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        None
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
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        if let Some(actions) = &launcher.actions {
            Some(actions.clone())
        } else {
            launcher
                .add_actions
                .as_ref()
                .map(|add_actions| add_actions.clone())
        }
    }
    #[inline(always)]
    fn has_actions(&self, _cx: &mut App) -> bool {
        false
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
                                    this.data = Some(Box::new(update));
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

impl Drop for PluginWidget {
    fn drop(&mut self) {
        self.subscribers.unregister(&self.tile_id);
    }
}

fn render_node(node: &PluginUiNode) -> AnyElement {
    match node {
        PluginUiNode::Container { style, children } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            el.children(children.iter().map(render_node))
                .into_any_element()
        }
        PluginUiNode::Text { content, style } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            el.child(StyledText::new(content.clone()))
                .into_any_element()
        }
        PluginUiNode::Icon { name, style } => {
            if let Some(icon) = resolve_icon_path(name) {
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
            }
        }
        PluginUiNode::Button { label, style } => {
            let mut el = div();
            style.apply_to_style_refinement(el.style());
            el.child(label.clone()).into_any_element()
        }
    }
}
