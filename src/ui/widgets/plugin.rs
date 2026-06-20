use std::{path::Path, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, AsyncApp, Entity, ImageSource, IntoElement, ParentElement, Styled, StyledText,
    WeakEntity, div, img,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig, LauncherId,
        plugin_launcher::{
            capabilities::PluginCapability,
            plugin_tile_state::PluginTileState,
            runtime::LuaRuntimeHandle,
            subscribers::{TileSubscribers, TileSubscribersGlobal},
            ui_schema::PluginUiNode,
        },
        utils::exec_mode::ExecMode,
        variant_type::LauncherType,
    },
    loader::{resolve_icon_path, utils::Priority},
    sherlock_msg,
    ui::{
        launcher::{
            LauncherView,
            context_menu::{ContextMenuAction, DynamicFunctionAction},
        },
        traits::RenderableChildImpl,
        utils::selection::Selection,
    },
    utils::errors::types::{PluginAction, SherlockErrorType},
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
        let LauncherType::Plugin(plg) = &launcher.launcher_type else {
            return None;
        };
        let caps = plg.capabilities;
        let path = plg.path.clone();
        let launcher_id = launcher.id();
        Some(Arc::from([Arc::new(ContextMenuAction::Fn(
            DynamicFunctionAction::new("Reload Plugin")
                .icon_name("sherlock-dev")
                .on_exec(move |cx| {
                    let path = path.clone();
                    cx.spawn(
                            move |weak_self: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                                let cx = cx.clone();
                                async move {
                                    reload_plugin(&launcher_id, path, caps, weak_self, cx).await
                                }
                            },
                        )
                        .detach();
                }),
        ))]))
    }
    #[inline(always)]
    fn has_actions(&self, _cx: &mut App) -> bool {
        true
    }
    #[inline(always)]
    fn vars(&self, _cx: &mut App) -> Option<&[crate::loader::utils::ExecVariable]> {
        None
    }
    #[inline(always)]
    fn increment_count(&self) {}
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

async fn reload_plugin(
    launcher_id: &LauncherId,
    path: Arc<Path>,
    caps: PluginCapability,
    weak_self: WeakEntity<LauncherView>,
    mut cx: AsyncApp,
) {
    let rt = LuaRuntimeHandle::get();
    let handle = match rt.load_plugin(path.clone(), caps).await {
        Ok(h) => h,
        Err(e) => {
            let _ = weak_self.update(&mut cx, |this, cx| {
                this.navigation.push_message(
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::Plugin(PluginAction::Load, path.display().to_string()),
                        e
                    ),
                    cx,
                );
            });
            return;
        }
    };

    let Ok(data_entity) = weak_self.read_with(&cx, |this, cx| {
        this.navigation.with_model(cx, |mdl| mdl.data())
    }) else {
        return;
    };

    data_entity.update(&mut cx, |data, cx| {
        let data_raw = Rc::make_mut(data);
        let Some(launcher) = data_raw.get_mut(launcher_id) else {
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
