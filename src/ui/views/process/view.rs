use crate::ui::views::{ShellCtx, SherlockView, list::list_body};
use gpui::{AnyElement, Context, Window};
use std::sync::Arc;

use gpui::App;

use crate::{launcher::LauncherConfig, ui::model::Model};

pub struct ProcessView {
    pub model: Model,
}

impl ProcessView {
    pub fn new(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        Self {
            model: Model::process(launcher, cx),
        }
    }
}

impl SherlockView for ProcessView {
    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        list_body(&self.model, shell, cx)
    }
}
