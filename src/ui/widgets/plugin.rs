use std::sync::Arc;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div};
use mlua::Table;

use crate::{
    app::theme::ThemeData,
    launcher::{Launcher, plugin_launcher::sandbox::PluginSandBox, utils::exec_mode::ExecMode},
    loader::utils::Priority,
    ui::{
        launcher::context_menu::ContextMenuAction, traits::RenderableChildImpl,
        utils::selection::Selection,
    },
};

#[derive(Clone)]
#[allow(unused)]
pub struct PluginWidget {
    pub sandbox: Arc<PluginSandBox>,
    pub tile: Table,
}

impl<'a> RenderableChildImpl<'a> for PluginWidget {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        _selection: Selection,
        _query: &str,
        _theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child("Test :)")
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
