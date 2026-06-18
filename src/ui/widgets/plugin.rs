use std::sync::Arc;

use gpui::{AnyElement, App, Entity, IntoElement, ParentElement, Styled, div};

use crate::{
    app::theme::ThemeData,
    launcher::{
        Launcher,
        plugin_launcher::{plugin_tile_state::PluginTileState, subscribers::TileSubscribers},
        utils::exec_mode::ExecMode,
    },
    loader::utils::Priority,
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
        _launcher: &Arc<Launcher>,
        _selection: Selection,
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
            return div().into_any_element();
        };

        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .text_color(theme.primary_text)
            .items_center()
            .child(data.title.clone())
            .children(data.subtitle.clone().map(|s| div().child(s)))
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>, _cx: &mut App) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<Launcher>, _cx: &mut App) -> Option<String> {
        None
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        "test"
    }
    #[inline(always)]
    fn actions(
        &self,
        _launcher: &Arc<Launcher>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        None
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
}

impl Drop for PluginWidget {
    fn drop(&mut self) {
        self.subscribers.unregister(&self.tile_id);
    }
}
