use std::sync::Arc;

use gpui::{AnyElement, App, Context, Entity, FocusHandle, SharedString, WeakEntity, Window, px};

use crate::{
    app::theme::ThemeData,
    ui::{
        launcher::{LauncherView, navigator::views::Layout},
        model::{Model, ResultsChange, data::ModelData},
        traits::HandleActions,
    },
};

pub mod emoji;
pub mod events;
pub mod file;
pub mod home;
pub mod list;
pub mod message;
pub mod process;
pub mod query;
pub mod settings;

pub use events::Propagation;

/// Which parts of the shared frame a view wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    pub search_bar: bool,
    pub mode_label: bool,
    pub status_bar: bool,
    pub sidebar: bool,
    /// Horizontal padding around the body (and sidebar). Off for pages that lay out the whole
    /// area themselves.
    pub inset: bool,
}

impl Chrome {
    pub const DEFAULT: Self = Self {
        search_bar: true,
        mode_label: true,
        status_bar: true,
        sidebar: true,
        inset: true,
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
        inset: false,
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

#[derive(Clone)]
pub struct Query {
    /// Lowercased search text.
    pub text: SharedString,
    /// Current mode (`"all"` or an alias)
    pub mode: SharedString,
    /// Re-run even if the query didn't change.
    pub force: bool,
    /// Compute the results right away instead of in the background.
    pub sync: bool,
    /// Notified when asynchronous results arrive: `LauncherView::results_changed`.
    pub shell: WeakEntity<LauncherView>,
}

pub enum QueryResult {
    /// The view already applied the new results to its model; the shell reacts now.
    Ready(ResultsChange),
    /// Results arrive later: the view applies them to its model, then calls
    /// `LauncherView::results_changed`.
    Pending,
    /// The view doesn't filter
    Ignored,
}

pub trait SherlockView: HandleActions + 'static + Sized {
    /// The data this view's model filters; `NoData` for views without items.
    type Data: ModelData;

    fn chrome(&self) -> Chrome {
        Chrome::DEFAULT
    }
    /// The view's content
    /// Initial scroll layout when the view is pushed: a row list by default.
    /// Where keyboard focus goes while this view is shown, e.g. a text field of a page.
    /// `None`: the search input if the view shows one, otherwise the launcher itself.
    fn focus_target(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    fn layout(&self) -> Layout {
        Layout::row(self.model().map_or(0, |m| m.len()), px(50.))
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement;

    /// Starts iltering for `query`. Views without a search (plain pages) keep the default.
    ///
    /// Must not update the shell synchronously (the shell is mid-update when this runs.), return
    /// `Ready` instead.
    fn on_query(&mut self, _query: &Query, _cx: &mut Context<Self>) -> QueryResult {
        QueryResult::Ignored
    }

    /// The view's model (selection, size, items). `None` for views without items.
    fn model(&self) -> Option<&Model<Self::Data>> {
        None
    }

    fn model_mut(&mut self) -> Option<&mut Model<Self::Data>> {
        None
    }
}
