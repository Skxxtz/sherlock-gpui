use std::sync::Arc;

use gpui::{App, AppContext, px};

use super::{Layout, NavView, ViewHandle, ViewKind};
use crate::{launcher::LauncherConfig, ui::model::process::view::ProcessView};

impl NavView {
    /// Create the process view, whick lists running processes.
    pub(crate) fn process(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let view = cx.new(|cx| ProcessView::new(launcher, cx));
        Self::new(
            ViewHandle::Process(view),
            Layout::row(0, px(50.)),
            ViewKind::Process,
        )
    }
}
