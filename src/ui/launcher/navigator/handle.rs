use std::sync::Arc;

use gpui::{AnyElement, AnyEntity, App, AppContext, Entity, FocusHandle, SharedString, Window, px};

use crate::{
    app::LauncherEntity,
    launcher::{LauncherConfig, variant_type::LauncherType},
    ui::{
        launcher::navigator::views::{Layout, NavView},
        model::{
            data::ModelData,
            item::{SelectionKey, ShellItem},
        },
        views::{
            Chrome, LauncherEvent, Propagation, Query, QueryResult, ShellCtx, SherlockView,
            emoji::EmojiView, file::view::FileView, home::HomeView, message::MessageView,
            process::view::ProcessView, settings::SettingsView,
        },
    },
};

/// Declares every view once: generates `ViewHandle`, `dispatch!` and `From<Entity<View>>`.
///
/// The leading `$` is passed through so the generated `dispatch!` can use its own
/// metavariables (`macro_rules!` can't write a literal `$` itself on stable).
macro_rules! views {
    ($d:tt ; $($variant:ident($view:ty)),* $(,)?) => {
        /// Typed handle to a view entity (static dispatch).
        #[derive(Clone)]
        pub enum ViewHandle {
            $($variant(Entity<$view>),)*
        }

        /// Runs `$body` with `$e` bound to the inner `&Entity<_>`, whatever the variant.
        macro_rules! dispatch {
            ($d handle:expr, $d e:ident => $d body:expr) => {
                match $d handle {
                    $(ViewHandle::$variant($d e) => $d body,)*
                }
            };
        }

        $(
            impl From<Entity<$view>> for ViewHandle {
                fn from(entity: Entity<$view>) -> Self {
                    Self::$variant(entity)
                }
            }
        )*
    };
}

// Adding a view = one line here (plus `impl SherlockView` on the view).
views! { $;
    Home(HomeView),
    Emoji(EmojiView),
    Files(FileView),
    Process(ProcessView),
    Settings(SettingsView),
    Messages(MessageView),
}

// Generic per-view helpers, called from `dispatch!` arms with the concrete view type.

fn on_results<V: SherlockView>(e: &Entity<V>, query: &SharedString, cx: &mut App) {
    let Some((data, visible)) = e
        .read(cx)
        .model()
        .map(|m| (m.data.read(cx).clone(), m.filtered_indices()))
    else {
        return;
    };
    data.on_results(&visible, query, cx);
}

fn refresh<V: SherlockView>(e: &Entity<V>, cx: &mut App) {
    if let Some(data) = e.read(cx).model().map(|m| m.data.read(cx).clone()) {
        data.refresh(cx);
    }
}

fn item_ref<'a, V: SherlockView>(
    e: &Entity<V>,
    n: usize,
    cx: &'a App,
) -> Option<&'a dyn ShellItem> {
    let model = e.read(cx).model()?;
    let id = *model.filtered.get(n)?;
    model
        .data
        .read(cx)
        .get(id)
        .map(|item| item as &dyn ShellItem)
}

fn with_item<V: SherlockView, R>(
    e: &Entity<V>,
    n: usize,
    cx: &mut App,
    f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
) -> Option<R> {
    let (data, id) = {
        let model = e.read(cx).model()?;
        (model.data.clone(), *model.filtered.get(n)?)
    };
    data.update(cx, |data, cx| data.get(id).map(|item| f(item, cx)))
}

impl ViewHandle {
    /// New results were applied: lets the data update per-query item state.
    pub fn on_results(&self, query: &SharedString, cx: &mut App) {
        dispatch!(self, e => on_results(e, query, cx))
    }

    /// Refreshes live items of this view's data.
    pub fn refresh(&self, cx: &mut App) {
        dispatch!(self, e => refresh(e, cx))
    }

    /// Number of visible items (0 without a model).
    pub fn len(&self, cx: &App) -> usize {
        dispatch!(self, e => e.read(cx).model().map_or(0, |m| m.len()))
    }

    /// Selected position, clamped to the visible items.
    pub fn selected_index(&self, cx: &App) -> usize {
        dispatch!(self, e => e.read(cx).model().map_or(0, |m| m.selected_index()))
    }

    /// Identity of the selected item.
    pub fn selected_key(&self, cx: &App) -> Option<SelectionKey> {
        dispatch!(self, e => e.read(cx).model().and_then(|m| m.selected_key()))
    }

    /// Position of the visible item with `key`.
    pub fn position_of(&self, key: SelectionKey, cx: &App) -> Option<usize> {
        dispatch!(self, e => e.read(cx).model().and_then(|m| m.position_of_key(key)))
    }

    /// The query that produced the current results, if any.
    pub fn last_query(&self, cx: &App) -> Option<SharedString> {
        dispatch!(self, e => e.read(cx).model().and_then(|m| m.last_query()))
    }

    /// Selects position `n`. Returns `false` if out of range or the view has no model.
    pub fn select(&self, n: usize, cx: &mut App) -> bool {
        dispatch!(self, e => e.update(cx, |v, _| v.model_mut().is_some_and(|m| m.select(n))))
    }

    /// Selects the visible item with `key`. Returns `false` if it isn't visible.
    pub fn select_key(&self, key: SelectionKey, cx: &mut App) -> bool {
        dispatch!(self, e => e.update(cx, |v, _| v.model_mut().is_some_and(|m| m.select_key(key))))
    }

    /// Back to the first item.
    pub fn reset_selection(&self, cx: &mut App) {
        dispatch!(self, e => e.update(cx, |v, _| {
            if let Some(m) = v.model_mut() {
                m.reset_selection();
            }
        }))
    }

    /// Forget the last query, so the next one always re-runs.
    pub fn clear_last_query(&self, cx: &mut App) {
        dispatch!(self, e => e.update(cx, |v, _| {
            if let Some(m) = v.model_mut() {
                m.last_query = None;
            }
        }))
    }

    /// Shared reference to the visible item at position `n`.
    pub fn item_ref<'a>(&self, n: usize, cx: &'a App) -> Option<&'a dyn ShellItem> {
        dispatch!(self, e => item_ref(e, n, cx))
    }

    /// Runs `f` with the visible item at position `n`. Updates the item's data entity, so `f`
    /// must not update that same entity.
    pub fn with_item<R>(
        &self,
        n: usize,
        cx: &mut App,
        f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
    ) -> Option<R> {
        dispatch!(self, e => with_item(e, n, cx, f))
    }

    /// Which frame parts this view wants.
    pub fn chrome(&self, cx: &App) -> Chrome {
        dispatch!(self, e => e.read(cx).chrome())
    }

    /// Where keyboard focus should go while this view is shown (see `SherlockView::focus_target`).
    pub fn focus_target(&self, cx: &App) -> Option<FocusHandle> {
        dispatch!(self, e => e.read(cx).focus_target(cx))
    }

    /// Initial scroll layout for a new navigator entry.
    pub fn layout(&self, cx: &App) -> Layout {
        dispatch!(self, e => e.read(cx).layout())
    }

    /// The view's body. Updates the view entity, so don't call it from inside that view.
    pub fn render_body(&self, shell: &ShellCtx, window: &mut Window, cx: &mut App) -> AnyElement {
        dispatch!(self, e => e.update(cx, |v, cx| v.render_body(shell, window, cx)))
    }

    /// Hands `query` to the view. Updates the view entity, so don't call it from inside that view.
    pub fn on_query(&self, query: &Query, cx: &mut App) -> QueryResult {
        dispatch!(self, e => e.update(cx, |v, cx| v.on_query(query, cx)))
    }

    /// Offers a launcher event to the view. Updates the view entity, so don't call it from inside
    /// that view.
    pub fn on_event(
        &self,
        event: &LauncherEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> Propagation {
        dispatch!(self, e => e.update(cx, |v, cx| v.on_event(event, window, cx)))
    }

    /// Type-erased entity, for rendering
    pub fn entity(&self) -> AnyEntity {
        dispatch!(self, e => e.clone().into())
    }
}

impl NavView {
    /// Dmenu: a home view over its own launcher entity.
    pub fn dmenu(entity: LauncherEntity, cx: &mut App) -> Self {
        Self::with_layout(
            cx.new(|cx| HomeView::new(entity, cx)),
            Layout::row(0, px(50.)),
        )
    }
}

/// Which kind of a view a `NavView` is, plis the data needed to build it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewKind {
    /// The permanent root view.
    Home,
    /// The error/warning/info overlay.
    Messages,
    /// Emoji picker.
    Emoji,
    /// Running processes.
    Process,
    /// Settings view.
    Settings,
    /// Dmenu mode: a home-styled view over its own launcher entity.
    Dmenu { entity: LauncherEntity },
    /// File browser, optionally starting in `dir`.
    Files { dir: Option<SharedString> },
}

impl ViewKind {
    /// Builds a pushable view. `Home` and `Messages` are fixed roots and owned by `Navigator`, so
    /// they return `None`
    pub fn build(&self, launcher: Arc<LauncherConfig>, cx: &mut App) -> Option<NavView> {
        Some(match self {
            Self::Emoji => NavView::new(cx.new(|cx| EmojiView::new(launcher, cx)), cx),
            Self::Process => NavView::new(cx.new(|cx| ProcessView::new(launcher, cx)), cx),
            Self::Files { dir } => {
                NavView::new(cx.new(|cx| FileView::new(launcher, dir.clone(), cx)), cx)
            }
            Self::Dmenu { entity } => NavView::dmenu(entity.clone(), cx),
            Self::Settings => NavView::new(cx.new(|_| SettingsView::new()), cx),
            Self::Home | Self::Messages => return None,
        })
    }
}

/// Returned when a launcher type has no view of its own.
pub struct NoView;

/// Maps a launcher type to the view it opens, if any.
impl TryFrom<&LauncherType> for ViewKind {
    type Error = NoView;
    fn try_from(value: &LauncherType) -> Result<Self, Self::Error> {
        match value {
            LauncherType::Emoji(_) => Ok(Self::Emoji),
            LauncherType::Process(_) => Ok(Self::Process),
            LauncherType::Files(fs) => Ok(Self::Files {
                dir: Some(fs.loc.clone()),
            }),
            _ => Err(NoView),
        }
    }
}
