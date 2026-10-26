use std::sync::Arc;

use gpui::{AnyElement, Context, Entity, SharedString, Window};

use crate::{
    app::theme::ThemeData,
    ui::launcher::{LauncherView, navigator::views::Layout},
};

pub mod emoji;
pub mod file;
pub mod home;
pub mod list;
pub mod message;
pub mod process;

/// Which parts of the shared frame a view wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    pub search_bar: bool,
    pub mode_label: bool,
    pub status_bar: bool,
    pub sidebar: bool,
}

impl Chrome {
    pub const DEFAULT: Self = Self {
        search_bar: true,
        mode_label: true,
        status_bar: true,
        sidebar: true,
    };

    /// Like `DEFAULT`, without the sidebar
    pub const NO_SIDEBAR: Self = Self {
        sidebar: false,
        ..Self::DEFAULT
    };

    /// A plain page: the body fills the whole window
    pub const NONE: Self = Self {
        search_bar: false,
        mode_label: false,
        status_bar: false,
        sidebar: false,
    };
}

impl Default for Chrome {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Shell state a view can use while rendering its body.
pub struct ShellCtx<'a> {
    /// For click handlers that act on the shell (select, execute, ...)
    pub shell: Entity<LauncherView>,
    pub query: SharedString,
    pub theme: Arc<ThemeData>,
    /// Scroll state of this view's navigator entry.
    pub layout: &'a Layout,
}

pub trait SherlockView: 'static + Sized {
    fn chrome(&self) -> Chrome {
        Chrome::DEFAULT
    }
    /// The view's content
    fn render_body(
        &mut self,
        shell: &ShellCtx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement;
}
