use crate::ui::views::{Query, QueryResult, query::launcher_query};
use crate::ui::views::{ShellCtx, SherlockView, list::list_body};
use gpui::{AnyElement, Context, Window};
use std::sync::Arc;

use gpui::{App, SharedString};

use crate::{launcher::LauncherConfig, ui::model::Model};

pub struct FileView {
    pub model: Model,
}

impl FileView {
    pub fn new(launcher: Arc<LauncherConfig>, dir: Option<SharedString>, cx: &mut App) -> Self {
        Self {
            model: Model::file_search(launcher, dir, cx),
        }
    }
}

impl SherlockView for FileView {
    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        list_body(&self.model, shell, cx)
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        launcher_query(&mut self.model, query, cx)
    }
}
