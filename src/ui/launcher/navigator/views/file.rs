use std::sync::Arc;

use gpui::{App, AppContext, SharedString, px};

use super::{Layout, NavView, ViewHandle, ViewKind};
use crate::{launcher::LauncherConfig, ui::views::file::view::FileView};

impl NavView {
    /// Creates a file browser view.
    /// `dir` is the starting directory; `None` lets `FileView` pick its default.
    pub(crate) fn files(
        launcher: Arc<LauncherConfig>,
        dir: Option<SharedString>,
        cx: &mut App,
    ) -> Self {
        let view = cx.new(|cx| FileView::new(launcher, dir.clone(), cx));
        Self::new(
            ViewHandle::Files(view),
            Layout::row(0, px(50.)),
            ViewKind::Files { dir },
        )
    }
}
