use std::sync::Arc;

use gpui::{AnyElement, App, AppContext, SharedString};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecEffect, LauncherConfig,
        utils::{binds::Bind, exec_mode::ExecMode},
        variant_type::InnerFunction,
    },
    loader::utils::{ExecVariable, Priority},
    ui::{launcher::context_menu::ContextMenuAction, utils::selection::Selection},
};

pub trait RenderableChildImpl<'a> {
    /// If set to true, disables the inheritage of the border and background fill of the list item
    const HANDLES_BORDERS: bool = false;
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: Selection,
        query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement;
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode>;
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority;
    fn search(&'a self, launcher: &Arc<LauncherConfig>) -> &'a str;
    /// Will only get called once the context menu gets opened
    fn actions(
        &self,
        launcher: &Arc<LauncherConfig>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        launcher.actions.clone()
    }
    /// Whether the `additional actions` indicator should show in the status bar
    fn has_actions(&self, _cx: &mut App) -> bool {
        false
    }
    fn binds(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<Arc<Vec<Bind>>> {
        None
    }
    fn execute_function(
        &self,
        _func: &InnerFunction,
        _launcher: &Arc<LauncherConfig>,
        _variables: &[(SharedString, SharedString)],
        _cx: &mut App,
    ) -> Option<ExecEffect> {
        None
    }
    fn based_show<C: AppContext>(&self, _keyword: &str, _cx: &mut C) -> Option<bool> {
        None
    }
    fn sidebar(&self, _cx: &mut App) -> Option<AnyElement> {
        None
    }
    fn update_sync(&self, _query: SharedString, _launcher: &Arc<LauncherConfig>, _cx: &mut App) {}
    fn update_async<C: AppContext>(&self, _launcher: Arc<LauncherConfig>, _cx: &mut C) {}
    fn vars(&self, _cx: &mut App) -> Option<&[ExecVariable]> {
        None
    }
    fn increment_count(&self) {}
    fn get_content(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String>;
}

// To make compatible with Boxed data
#[allow(dead_code)]
pub trait HandlesBorders {
    const HANDLES_BORDERS: bool;
}

impl<T> HandlesBorders for Box<T>
where
    for<'a> T: RenderableChildImpl<'a>,
{
    const HANDLES_BORDERS: bool = <T as RenderableChildImpl<'_>>::HANDLES_BORDERS;
}
