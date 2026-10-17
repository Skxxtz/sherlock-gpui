use std::sync::Arc;

use gpui::{AnyEntity, App, AppContext, Entity, SharedString};

use crate::{
    app::LauncherEntity,
    launcher::{LauncherConfig, variant_type::LauncherType},
    ui::{
        launcher::navigator::views::NavView,
        model::{
            Model, emoji::EmojiView, file::view::FileView, home::HomeView, message::MessageView,
            process::view::ProcessView,
        },
    },
};

/// Typed handle to a view entity. Adding a view = one variant + one line in `dispatch!`.
#[derive(Clone)]
pub enum ViewHandle {
    Home(Entity<HomeView>),
    Emoji(Entity<EmojiView>),
    Files(Entity<FileView>),
    Process(Entity<ProcessView>),
    Messages(Entity<MessageView>),
}

/// Runs `$body` with `$e` bound to the inner `&Entity<_>`, whatever the variant.
macro_rules! dispatch {
    ($handle:expr, $e:ident => $body:expr) => {
        match $handle {
            ViewHandle::Home($e) => $body,
            ViewHandle::Emoji($e) => $body,
            ViewHandle::Files($e) => $body,
            ViewHandle::Process($e) => $body,
            ViewHandle::Messages($e) => $body,
        }
    };
}

impl ViewHandle {
    /// Read this view's model
    pub fn with_model<R>(&self, cx: &App, f: impl FnOnce(&Model) -> R) -> R {
        dispatch!(self, e => f(&e.read(cx).model))
    }

    /// Mutate this view's model through an entity update.
    /// Don't update the same entity again inside `f`, or GPUI panics.
    pub fn with_model_mut<R, C: AppContext>(
        &self,
        cx: &mut C,
        f: impl FnOnce(&mut Model, &mut App) -> R,
    ) -> R {
        dispatch!(self, e => e.update(cx, |v, cx| f(&mut v.model, cx)))
    }

    /// Type-erased entity, for rendering
    pub fn entity(&self) -> AnyEntity {
        dispatch!(self, e => e.clone().into())
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
            Self::Emoji => NavView::emoji(launcher, cx),
            Self::Process => NavView::process(launcher, cx),
            Self::Files { dir } => NavView::files(launcher, dir.clone(), cx),
            Self::Dmenu { entity } => NavView::dmenu(entity.clone(), cx),
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
