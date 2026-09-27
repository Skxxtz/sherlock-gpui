use std::sync::Arc;

use gpui::{AnyElement, App, AppContext, SharedString};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecEffect,
        utils::{binds::Bind, exec_mode::ExecMode},
        variant_type::InnerFunction,
    },
    loader::utils::ExecVariable,
    ui::{launcher::context_menu::ContextMenuAction, utils::selection::Selection},
    utils::errors::SherlockMessage,
};

pub trait RenderableChildDelegate<'a> {
    /// Whether the child internally applies style for borders
    fn handles_borders(&self) -> bool;

    /// The logic to render the widget
    fn render(
        &self,
        selection: Selection,
        query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement;

    /// Generates an execution path based on the child and the context menu action
    fn build_action_exec(&'a self, action: Arc<ContextMenuAction>, cx: &mut App) -> ExecMode;

    /// Generates an execution path when pressing return on this widget
    fn build_exec(&self, cx: &mut App) -> Option<ExecMode>;

    /// The string that contains or otherwise matces the user-provided search query
    fn search(&'a self) -> &'a str;

    /// The variable fields that should be shown next to the search input
    fn vars(&self, cx: &mut App) -> Option<&[ExecVariable]>;

    /// The context menu actions for this widget. (Gets called on the selected item only if:
    /// self.has_actions == true and the context menu gets opened)
    fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>>;

    /// Whether this widget owns any context menu actions. (This gets called only on the selected
    /// item)
    fn binds(&self, _cx: &mut App) -> Option<Arc<Vec<Bind>>>;

    /// Execute inner functions
    fn execute_function(
        &self,
        func: InnerFunction,
        variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage>;

    /// Get inner binds for a launcher and its children
    fn has_actions(&self, cx: &mut App) -> bool;

    /// Boolean logic for conditional display (e.g., calculator)
    fn based_show<C: AppContext>(&self, keyword: &str, cx: &mut C) -> Option<bool>;

    /// Sidebar rendering
    fn sidebar(&self, cx: &mut App) -> Option<AnyElement>;

    /// Sync update on every keypress
    fn update_sync(&self, query: SharedString, cx: &mut App);

    /// Updates a dynamic renderable child that requires re-evaluation.
    ///
    /// This is used for items whose state depends on internal logic (e.g., a timer)
    /// or external factors (e.g., a weather API or file system change).
    fn update_async<C: AppContext>(&self, cx: &mut C);

    /// Updates the execution count for supported children
    fn increment_count(&self);

    /// Lets the selected item handle a navigation key itself (e.g. move
    /// between items inside a tile). Return `true` if handled; otherwise
    /// the launcher moves to the next/previous row.
    fn move_inner(
        &self,
        direction: &crate::ui::launcher::views::MoveDirection,
        cx: &mut App,
    ) -> bool;

    /// Called when the selection leaves this item: drop any inner focus
    fn reset_inner(&self, cx: &mut App);

    /// Gets content to be copied, or printet on return
    fn get_content(&self, cx: &mut App) -> Option<String>;
}
