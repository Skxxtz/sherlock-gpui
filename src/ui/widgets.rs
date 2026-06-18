use gpui::{AnyElement, App, AppContext, SharedString};
use std::sync::Arc;

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
    loader::utils::{ExecVariable, Priority},
    ui::{
        launcher::context_menu::ContextMenuAction,
        traits::{RenderableChildDelegate, RenderableChildImpl},
        utils::selection::Selection,
        widgets::{
            audio::MusicPlayerWidget, clipboard::ClipWidget, dmenu::DmenuData, event::EventWidget,
            message::MessageChild, plugin::PluginWidget, process::ProcessData, script::ScriptData,
            timer::TimerChild, translator::TranslationData, weather::WeatherWidget,
        },
    },
    utils::{config::HomeType, errors::SherlockMessage},
};

use calculator::CalcData;
use file::FileData;
use theme::ThemeWidget;

/// Creates enum RenderableChild,
/// ## Example:
/// ```
/// renderable_enum! {
///     enum RenderableChild {
///         App(AppData),
///         Weather(WeatherData),
///     }
/// }
/// ```
macro_rules! renderable_enum {
    (
        enum $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
        }
    ) => {
        #[derive(Clone)]
        pub enum $name {
            $(
                $variant {
                    launcher: Arc<LauncherConfig>,
                    inner: $inner,
                }
            ),*
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$variant { .. } => write!(f, "{}", stringify!($variant)),
                    )*
                }
            }
        }

        impl<'a> RenderableChildDelegate<'a> for $name {
            fn handles_borders(&self) -> bool {
                match self {
                    $(Self::$variant { .. } => <$inner>::HANDLES_BORDERS),*
                }
            }

            fn render(&self, selection: Selection, query: &str, theme: Arc<ThemeData>, cx: &mut App) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, selection, query, theme, cx)),*
                }
            }

            fn build_action_exec(&self, action: Arc<ContextMenuAction>, cx: &mut App) -> ExecMode {
                ExecMode::from_app_action(action, &self, cx)
            }

            fn build_exec(&self, cx: &mut App) -> Option<ExecMode> {
                match self {
                    $(Self::$variant {launcher, inner} => {
                        inner.build_exec(launcher, cx)
                    }),*
                }
            }

            fn search(&'a self) -> &'a str {
                match self {
                    $(Self::$variant {inner, launcher} => inner.search(launcher)),*
                }
            }

            fn get_content(&self, cx: &mut App) -> Option<String> {
                match self {
                    $(Self::$variant {launcher, inner} => {
                        inner.get_content(launcher, cx)
                    }),*
                }
            }

            fn vars(&self, cx: &mut App) -> Option<&[ExecVariable]> {
                match self {
                    $(Self::$variant {inner, ..} => inner.vars(cx)),*
                }
            }

            fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
                match self {
                    $(Self::$variant {inner, launcher} => inner.actions(launcher, cx)),*
                }
            }

            fn has_actions(&self, cx: &mut App) -> bool {
                match self {
                    $(Self::$variant {inner, launcher} => {
                        if launcher.actions.as_ref().map_or(false, |actions| !actions.is_empty()) {
                            return true
                        }
                        if launcher.add_actions.as_ref().map_or(false, |actions| !actions.is_empty()) {
                            return true
                        }
                        inner.has_actions(cx)
                    }),*
                }
            }

            fn binds(&self, cx: &mut App) -> Option<Arc<Vec<Bind>>> {
                match self {
                    $(Self::$variant {inner, launcher} => inner.binds(launcher, cx)),*
                }
            }

            fn execute_function(&self, func: InnerFunction, variables: &[(SharedString, SharedString)], cx: &mut App) -> Result<ExecEffect, SherlockMessage> {
                match self {
                    $(
                        Self::$variant {inner, launcher} => {
                            if let Some(first) = inner.execute_function(&func, launcher, variables, cx) {
                                return Ok(first)
                            }
                            launcher.launcher_type.execute_function(func, self, variables, cx)
                        }
                    ),*
                }
            }

            fn based_show<C: AppContext>(&self, keyword: &str, cx: &mut C) -> Option<bool> {
                match self {
                    $(Self::$variant {inner, ..} => inner.based_show(keyword, cx)),*
                }
            }

            fn sidebar(&self, cx: &mut App) -> Option<AnyElement> {
                match self {
                    $(Self::$variant {inner, ..} => inner.sidebar(cx)),*
                }
            }

            fn update_sync(&self, query: SharedString, cx: &mut App) {
                match self {
                    $(Self::$variant {inner, launcher} => inner.update_sync(query, launcher, cx)),*
                }
            }

            fn update_async<C: AppContext>(&self,  cx: &mut C) {
                match self {
                    $(Self::$variant {inner, launcher} => inner.update_async(launcher.clone(), cx)),*
                }
            }

            fn increment_count(&self) {
                match self {
                    $(Self::$variant {inner, ..} => inner.increment_count()),*
                }
            }
        }

        impl<'a> LauncherValues<'a> for $name {
            fn name(&'a self) -> Option<&'a str> {
                self.launcher_config().name.as_ref().map(|s| s.as_str())
            }

            fn home(&self) -> HomeType {
                self.launcher_config().home
            }

            fn is_async(&self) -> bool {
                if matches!(
                    self.launcher_variant(),
                    LauncherVariant::Weather
                        | LauncherVariant::MusicPlayer
                        | LauncherVariant::Clipboard
                        | LauncherVariant::Event
                ) {
                    return true;
                }

                match self.launcher_type() {
                    LauncherType::Script(scr) => scr.r#async,
                    _ => false,
                }
            }

            fn alias(&'a self) -> Option<&'a str> {
                self.launcher_config().alias.as_deref()
            }

            fn priority(&self) -> Priority {
                match self {
                    $(Self::$variant {inner, launcher} => inner.priority(launcher)),*
                }
            }

            fn spawn_focus(&self) -> bool {
                self.launcher_config().spawn_focus
            }

            fn launcher_type(&self) -> &LauncherType {
                &self.launcher_config().launcher_type
            }

            fn launcher_variant(&self) -> LauncherVariant {
                self.launcher_config().launcher_type.as_ref().into()
            }

            fn shortcut(&self) -> bool {
                self.launcher_config().shortcut
            }
        }

        impl <'a> $name {
            #[inline(always)]
            fn launcher_config(&'a self) -> &'a LauncherConfig {
                match self {
                    $(Self::$variant {launcher, ..} => &launcher),*
                }
            }

            pub fn with_launcher<F, R>(&self, f: F) -> R
            where
                F: FnOnce(&Arc<LauncherConfig>) -> R
            {
                match self {
                    $(Self::$variant { launcher, .. } => f(launcher)),*
                }
            }
        }

    };
}

renderable_enum! {
    enum RenderableChild {
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
}

impl RenderableChild {
    pub fn get_exec(&self) -> Option<String> {
        match self {
            Self::App { inner, launcher } => inner.get_exec(launcher),
            _ => None,
        }
    }
}
