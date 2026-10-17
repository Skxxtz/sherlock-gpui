use gpui::{App, AppContext, px};
use std::sync::Arc;

use super::{Layout, NavView, ViewHandle, ViewKind};
use crate::{launcher::LauncherConfig, ui::model::emoji::EmojiView};

/// Number of emojis per row in the grid
const COLUMNS: usize = 6;

impl NavView {
    /// Creates the emoji picker, shown as a grid
    pub(crate) fn emoji(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let view = cx.new(|cx| EmojiView::new(launcher, cx));
        let len = view.read(cx).model.len();
        Self::new(
            ViewHandle::Emoji(view),
            Layout::grid(len, px(100.), COLUMNS),
            ViewKind::Emoji,
        )
    }
}
