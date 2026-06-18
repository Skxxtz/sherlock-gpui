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
