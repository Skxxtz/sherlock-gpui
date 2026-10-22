use gpui::{AnyElement, App, AppContext, SharedString};
use std::{fmt, sync::Arc};

pub mod app;
pub mod audio;
pub mod calculator;
pub mod clipboard;
pub mod dmenu;
pub mod emoji;
pub mod event;
pub mod file;
pub mod message;
pub mod plugin;
pub mod process;
pub mod script;
pub mod theme;
pub mod timer;
pub mod translator;
pub mod weather;

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecEffect, LauncherConfig, LauncherValues,
        app_launcher::app_data::AppData,
        emoji_launcher::EmojiData,
        utils::{binds::Bind, exec_mode::ExecMode},
        variant_type::{InnerFunction, LauncherType, LauncherVariant},
    },
    loader::utils::Priority,
    ui::{
        launcher::{
            context_menu::{ContextMenuAction, quick_actions::Hideable},
            navigator::views::MoveDirection,
        },
        runtime_arguments::RuntimeArgument,
        traits::{RenderableChildDelegate, RenderableChildImpl},
        utils::selection::Selection,
        widgets::{
            audio::MusicPlayerWidget, clipboard::ClipWidget, dmenu::DmenuData, event::EventWidget,
            message::MessageChild, plugin::PluginWidget, process::ProcessData, script::ScriptData,
            timer::TimerChild, translator::TranslationData, weather::WeatherWidget,
        },
    },
    utils::{config::HomeType, errors::SherlockMessage, item_key::ItemKey},
};

use calculator::CalcData;
use file::FileData;
use theme::ThemeWidget;

/// Single source of truth for child variants. Generates:
/// - `enum ChildKind` (the per-variant payload; the launcher lives in `RenderableChild`)
/// - `From<Inner> for ChildKind` for every variant
/// - `dispatch!(kind, inner => expr)`, an exhaustive match binding the payload
///
/// `$d` is a literal `$`, passed in so the nested macro can declare its own metavariables.
macro_rules! child_kinds {
    ($d:tt $($variant:ident($inner:ty)),* $(,)?) => {
        #[derive(Clone)]
        pub enum ChildKind {
            $($variant($inner)),*
        }

        $(impl From<$inner> for ChildKind {
            #[inline]
            fn from(v: $inner) -> Self { Self::$variant(v) }
        })*

        macro_rules! dispatch {
            ($d kind:expr, $d i:ident => $d body:expr) => {
                match $d kind {
                    $(ChildKind::$variant($d i) => $d body),*
                }
            };
        }
    };
}

child_kinds! { $
    App(AppData),
    Calc(CalcData),
    Clip(ClipWidget),
    Emoji(EmojiData),
    Event(EventWidget),
    File(FileData),
    Message(MessageChild),
    Music(MusicPlayerWidget),
    Plugin(PluginWidget),
    Process(ProcessData),
    Script(ScriptData),
    Theme(ThemeWidget),
    Timer(TimerChild),
    Translator(TranslationData),
    Weather(WeatherWidget),
    Dmenu(DmenuData),
}

/// A launcher config paired with its rendered payload.
#[derive(Clone)]
pub struct RenderableChild {
    pub id: Option<ItemKey>,
    pub launcher: Arc<LauncherConfig>,
    pub kind: ChildKind,
}

impl RenderableChild {
    pub fn new(launcher: Arc<LauncherConfig>, inner: impl Into<ChildKind>) -> Self {
        let kind: ChildKind = inner.into();
        let id = ItemKey::of(&kind, &launcher);
        Self { id, launcher, kind }
    }

    pub fn get_exec(&self) -> Option<String> {
        match &self.kind {
            ChildKind::App(app) => app.get_exec(&self.launcher),
            _ => None,
        }
    }

    pub fn item_key(&self) -> Option<ItemKey> {
        self.id.clone()
    }

    pub fn item_key_ref(&self) -> Option<&ItemKey> {
        self.id.as_ref()
    }
}

impl fmt::Debug for RenderableChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = dispatch!(&self.kind, i => std::any::type_name_of_val(i));
        f.debug_struct("RenderableChild")
            .field("kind", &kind.rsplit("::").next().unwrap_or(kind))
            .field("name", &self.launcher.name)
            .finish()
    }
}

impl RenderableChildDelegate for RenderableChild {
    fn handles_borders(&self) -> bool {
        dispatch!(&self.kind, i => i.handles_borders())
    }

    fn render(
        &self,
        selection: Selection,
        query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        dispatch!(&self.kind, i => i.render(&self.launcher, selection, query, theme, cx))
    }

    fn build_action_exec(&self, action: Arc<ContextMenuAction>, cx: &mut App) -> ExecMode {
        ExecMode::from_app_action(action, self, cx)
    }

    fn build_exec(&self, cx: &mut App) -> Option<ExecMode> {
        dispatch!(&self.kind, i => i.build_exec(&self.launcher, cx))
    }

    fn search(&self) -> &str {
        dispatch!(&self.kind, i => i.search(&self.launcher))
    }

    fn get_content(&self, cx: &mut App) -> Option<String> {
        dispatch!(&self.kind, i => i.get_content(&self.launcher, cx))
    }

    fn vars(&self, cx: &mut App) -> Option<&[RuntimeArgument]> {
        dispatch!(&self.kind, i => i.vars(cx))
    }

    fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        dispatch!(&self.kind, i => i.actions(&self.launcher, cx))
    }

    fn has_actions(&self, cx: &mut App) -> bool {
        let l = &self.launcher;
        l.actions.as_ref().is_some_and(|a| !a.is_empty())
            || l.add_actions.as_ref().is_some_and(|a| !a.is_empty())
            || dispatch!(&self.kind, i => i.has_actions(cx))
    }

    fn binds(&self, cx: &mut App) -> Option<Arc<Vec<Bind>>> {
        dispatch!(&self.kind, i => i.binds(&self.launcher, cx))
    }

    fn execute_function(
        &self,
        func: InnerFunction,
        variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        let first =
            dispatch!(&self.kind, i => i.execute_function(&func, &self.launcher, variables, cx));
        match first {
            Some(effect) => Ok(effect),
            None => self
                .launcher
                .launcher_type
                .execute_function(func, self, variables, cx),
        }
    }

    fn based_show<C: AppContext>(&self, keyword: &str, cx: &mut C) -> Option<bool> {
        dispatch!(&self.kind, i => i.based_show(keyword, cx))
    }

    fn sidebar(&self, cx: &mut App) -> Option<AnyElement> {
        dispatch!(&self.kind, i => i.sidebar(&self.launcher, cx))
    }

    fn has_sidebar(&self) -> bool {
        dispatch!(&self.kind, i => i.has_sidebar())
    }

    fn update_sync(&self, query: SharedString, cx: &mut App) {
        dispatch!(&self.kind, i => i.update_sync(query, &self.launcher, cx))
    }

    fn update_async<C: AppContext>(&self, cx: &mut C) {
        dispatch!(&self.kind, i => i.update_async(self.launcher.clone(), cx))
    }

    fn increment_count(&self) {
        dispatch!(&self.kind, i => i.increment_count())
    }

    fn move_inner(&self, direction: &MoveDirection, cx: &mut App) -> bool {
        dispatch!(&self.kind, i => i.move_inner(direction, cx))
    }

    fn reset_inner(&self, cx: &mut App) {
        dispatch!(&self.kind, i => i.reset_inner(cx))
    }
}

impl<'a> LauncherValues<'a> for RenderableChild {
    fn name(&'a self) -> Option<&'a str> {
        self.launcher.name.as_deref()
    }

    fn home(&self) -> HomeType {
        self.launcher.home
    }

    fn is_async(&self) -> bool {
        match &self.kind {
            ChildKind::Weather(_)
            | ChildKind::Music(_)
            | ChildKind::Clip(_)
            | ChildKind::Event(_) => true,
            ChildKind::Script(_) => {
                matches!(&self.launcher.launcher_type, LauncherType::Script(s) if s.r#async)
            }
            _ => false,
        }
    }

    fn alias(&'a self) -> Option<&'a str> {
        self.launcher.alias.as_deref()
    }

    fn priority(&self) -> Priority {
        dispatch!(&self.kind, i => i.priority(&self.launcher))
    }

    fn spawn_focus(&self) -> bool {
        self.launcher.spawn_focus
    }

    fn sidebar_default_on(&self) -> bool {
        self.launcher.sidebar_default_on
    }

    fn launcher_type(&self) -> &LauncherType {
        &self.launcher.launcher_type
    }

    fn launcher_variant(&self) -> LauncherVariant {
        self.launcher.launcher_type.as_ref().into()
    }

    fn shortcut(&self) -> bool {
        self.launcher.shortcut
    }
}

impl Hideable for RenderableChild {
    fn hide_id(&self) -> Option<String> {
        dispatch!(&self.kind, i => Hideable::hide_id(i))
    }

    fn is_hideable(&self) -> bool {
        dispatch!(&self.kind, i => Hideable::is_hideable(i))
    }
}
