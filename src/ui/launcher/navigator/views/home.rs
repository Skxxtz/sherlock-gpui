use gpui::{App, AppContext, px};

use crate::{
    app::LauncherEntity,
    ui::{
        launcher::navigator::{
            handle::{ViewHandle, ViewKind},
            views::{Layout, NavView},
        },
        model::home::HomeView,
    },
};

impl NavView {
    /// Creates the home view, the permanent root of the navigator.
    /// `len` is the initial item count for the list.
    pub(crate) fn home(entity: LauncherEntity, len: usize, cx: &mut App) -> Self {
        let view = cx.new(|cx| HomeView::new(entity, cx));
        Self::new(
            ViewHandle::Home(view),
            Layout::row(len, px(48.)),
            ViewKind::Home,
        )
    }

    /// Creates a dmenu view, which reuses `HomeView` with its own launcher entity.
    pub(crate) fn dmenu(entity: LauncherEntity, cx: &mut App) -> Self {
        let view = cx.new(|cx| HomeView::new(entity.clone(), cx));
        Self::new(
            ViewHandle::Home(view),
            Layout::row(0, px(50.)),
            ViewKind::Dmenu { entity },
        )
    }
}
