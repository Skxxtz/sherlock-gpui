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
