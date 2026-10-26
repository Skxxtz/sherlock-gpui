use std::sync::Arc;

use gpui::{AnyElement, App, AppContext, SharedString};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecEffect, LauncherConfig,
        utils::{binds::Bind, exec_mode::ExecMode},
        variant_type::InnerFunction,
    },
    ui::{
        launcher::{context_menu::ContextMenuAction, navigator::views::MoveDirection},
        runtime_arguments::RuntimeArgument,
        utils::selection::Selection,
    },
};

pub trait RenderableChildImpl {
    /// If set to true, disables the inheritage of the border and background fill of the list item
    const HANDLES_BORDERS: bool = false;
    fn handles_borders(&self) -> bool {
        Self::HANDLES_BORDERS
    }
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: Selection,
        query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement;
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode>;
    fn search(&self, launcher: &Arc<LauncherConfig>) -> &str;
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
    fn has_based_show(&self) -> bool {
        false
    }
    fn sidebar(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<AnyElement> {
        None
    }
    fn has_sidebar(&self) -> bool {
        false
    }
    /// Lets the selected item handle a navigation key itself (e.g. move
    /// between items inside a tile). Return `true` if handled; otherwise
    /// the launcher moves to the next/previous row.
    fn move_inner(&self, _direction: &MoveDirection, _cx: &mut App) -> bool {
        false
    }
    /// Called when the selection leaves this item: drop any inner focus.
    fn reset_inner(&self, _cx: &mut App) {}
    fn update_sync(&self, _query: SharedString, _launcher: &Arc<LauncherConfig>, _cx: &mut App) {}
    fn update_async<C: AppContext>(&self, _launcher: Arc<LauncherConfig>, _cx: &mut C) {}
    fn vars(&self, _cx: &mut App) -> Option<&[RuntimeArgument]> {
        None
    }
    fn get_content(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String>;
}
