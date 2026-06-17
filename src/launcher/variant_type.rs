use gpui::{App, SharedString};
use md_rs::components::{ParentComponentExt, container::Container};
use std::mem;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    docs::{Documentation, launcher::LauncherDoc},
    launcher::{
        Bind, ExecEffect, LauncherProvider,
        app_launcher::AppLauncher,
        audio_launcher::{MusicPlayerFunctions, MusicPlayerLauncher},
        bookmark_launcher::BookmarkLauncher,
        calc_launcher::CalculatorLauncher,
        category_launcher::CategoryLauncher,
        clipboard_launcher::ClipboardLauncher,
        debug_launcher::{DebugFunctions, DebugLauncher},
        dmenu_launcher::DmenuLauncher,
        emoji_launcher::EmojiPicker,
        event_launcher::{EventLauncher, EventLauncherFunctions},
        file_launcher::FileLauncher,
        message_launcher::MessageLauncher,
        plugin_launcher::PluginLauncher,
        process_launcher::{ProcessLauncher, ProcessLauncherFunctions},
        script_launcher::{ScriptFunctions, ScriptLauncher},
        system_cmd_launcher::CommandLauncher,
        theme_launcher::{ThemePicker, ThemePickerFunctions},
        timer_launcher::{TimerLauncher, TimerLauncherFunctions},
        translator_launcher::Translator,
        weather_launcher::WeatherLauncher,
        web_launcher::WebLauncher,
    },
    loader::utils::RawLauncher,
    ui::widgets::RenderableChild,
    utils::errors::SherlockMessage,
};

macro_rules! create_variants {
    (
        enum $name:ident {
            $( $variant:ident( $inner:ty $(, $extra:ty)* ) ),* $(,)?
        }
    ) => {
        //trait enforced
        #[allow(dead_code)]
        fn _assert_launcher_docs() {
            fn assert_launcher_doc<T: LauncherDoc>() {}
            $(
                assert_launcher_doc::<$inner>();
            )*
        }

        #[derive(Clone, Debug, Default)]
        pub enum $name {
            $($variant($inner),)*
            #[default]
            Empty,
        }

        #[derive(Deserialize, Debug, Serialize, Clone, Copy, Default, Display, PartialEq, Eq, Hash)]
        #[serde(rename_all = "snake_case")]
        #[strum(serialize_all = "snake_case")]
        pub enum LauncherVariant {
            $($variant,)*
            #[default]
            Empty,
        }

        #[derive(Clone, Debug, PartialEq)]
        pub enum InnerFunction {
            $(
                $( $variant($extra), )?
            )*

            #[allow(dead_code)]
            SelectionUp,
            #[allow(dead_code)]
            SelectionDown,
            #[allow(dead_code)]
            Empty
        }

        impl InnerFunction {
            pub fn from_str(variant: &$name, func_name: &str) -> Self {
                match func_name {
                    "selection_up" => return Self::SelectionUp,
                    "selection_down" => return Self::SelectionDown,
                    _ => {}
                }

                match variant {
                    $(
                        $name::$variant(_) => {
                            $(
                                use std::str::FromStr;
                                if let Ok(f) = <$extra>::from_str(func_name) {
                                    return Self::$variant(f);
                                }
                            )?
                            Self::Empty
                        }
                    )*
                    $name::Empty => Self::Empty,
                }
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                mem::discriminant(self) == mem::discriminant(other)
            }
        }

        impl Documentation for LauncherType {
            type Docs = Container;
            fn docs() -> Self::Docs {
                Container::default().children(
                    [ $( <$inner>::doc(),)* ].iter().map(Container::from)
                )
            }
        }

        impl $name {
            pub fn get_render_obj(
                &self,
                launcher: std::sync::Arc<crate::launcher::Launcher>,
                ctx: &crate::loader::LoadContext,
                opts: std::sync::Arc<serde_json::Value>,
                messages: &mut Vec<SherlockMessage>,
                cx: &mut App
            ) -> Result<Vec<RenderableChild>, SherlockMessage> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::objects(inner, launcher, ctx, opts, messages, cx),
                    )*
                    Self::Empty => Ok(vec![]),
                }
            }
            pub fn binds(&self) -> Option<Arc<Vec<Bind>>> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::binds(inner),
                    )*
                    Self::Empty => None
                }
            }
            pub fn execute_function(
                &self,
                func: InnerFunction,
                child: &RenderableChild,
                variables: &[(SharedString, SharedString)],
                cx: &mut App,
            ) -> Result<ExecEffect, SherlockMessage> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::execute_function(inner, func, child, variables, cx),
                    )*
                    Self::Empty => unimplemented!(),
                }
            }
        }

        impl LauncherVariant {
            #[cfg(test)]
            pub fn supports_functions(&self) -> bool {
                match self {
                    $(
                        Self::$variant => {
                            let _has_extra = false;
                            $( let _has_extra = { let _ = std::marker::PhantomData::<$extra>::default(); true }; )?
                            _has_extra
                        }
                    )*
                    Self::Empty => false,
                }
            }
            pub fn into_launcher_type(self, raw: &RawLauncher) -> $name {
                match self {
                    $(
                        Self::$variant => <$inner as LauncherProvider>::parse(raw),
                    )*
                    Self::Empty => $name::Empty
                }
            }
        }

        impl From<&$name> for LauncherVariant {
            fn from(t: &$name) -> Self {
                match t {
                    $(
                        $name::$variant(_) => Self::$variant,
                    )*
                    $name::Empty => Self::Empty,
                }
            }
        }

        impl From<$name> for LauncherVariant {
            fn from(t: $name) -> Self {
                From::from(&t)
            }
        }

        impl AsRef<$name> for $name {
            fn as_ref(&self) -> &$name {
                self
            }
        }

        #[cfg(test)]
        mod launcher_doc_tests {
            use super::*;
            use crate::docs::launcher::LauncherDocEntry;

            #[tokio::test]
            async fn test_all_docs_valid() {
                let pairs: &[(LauncherVariant, fn() -> LauncherDocEntry)] = &[
                    $(
                        (LauncherVariant::$variant, <$inner>::doc),
                    )*
                ];

                for (var, doc_fn) in pairs {
                    let doc = doc_fn();

                    // check functions match
                    if var.supports_functions() {
                        assert!(
                            !doc.inner_functions.is_empty(),
                            "{:?} supports functions but doc lists none", var
                        );
                    } else {
                        assert!(
                            doc.inner_functions.is_empty(),
                            "{:?} has no functions but doc lists some", var
                        );
                    }

                    // parse every example
                    for example in doc.examples {
                        let raw: RawLauncher = serde_json::from_str(example.json)
                            .unwrap_or_else(|e| panic!(
                                "{:?} example '{}' is not valid RawLauncher: {}", var, example.description, e
                            ));
                        let launcher = var.into_launcher_type(&raw);
                        assert!(
                            !matches!(launcher, LauncherType::Empty),
                            "{:?} example '{}' parsed to Empty — args schema mismatch", var, example.description
                        );
                    }
                }
            }
        }
    };
}

create_variants! {
    enum LauncherType {
        Apps(AppLauncher),
        Bookmarks(BookmarkLauncher),
        Calculator(CalculatorLauncher),
        Categories(CategoryLauncher),
        Clipboard(ClipboardLauncher),
        Commands(CommandLauncher),
        Debug(DebugLauncher, DebugFunctions),
        Dmenu(DmenuLauncher),
        Emoji(EmojiPicker),
        Event(EventLauncher, EventLauncherFunctions),
        Files(FileLauncher),
        Message(MessageLauncher),
        MusicPlayer(MusicPlayerLauncher, MusicPlayerFunctions),
        Plugin(PluginLauncher),
        Process(ProcessLauncher, ProcessLauncherFunctions),
        Script(ScriptLauncher, ScriptFunctions),
        Theme(ThemePicker, ThemePickerFunctions),
        Timer(TimerLauncher, TimerLauncherFunctions),
        Translator(Translator),
        Weather(WeatherLauncher),
        Web(WebLauncher),
        // Integrate later: TODO
        // Pipe(PipeLauncher),
    }
}

#[macro_export]
macro_rules! ensure_func {
    ($val:expr, $variant:path) => {
        if let $variant(inner) = $val {
            inner
        } else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidFunction,
                format!("Invalid function {:?} for this launcher", $val)
            ));
        }
    };
}
#[macro_export]
macro_rules! skip_func_if_nav {
    ($val:expr) => {
        if matches!(
            $val,
            InnerFunction::SelectionUp | InnerFunction::SelectionDown
        ) {
            return Ok(ExecEffect::None);
        }
    };
}

#[macro_export]
macro_rules! variant_name {
    ($variant:ident) => {{
        const _: $crate::launcher::variant_type::LauncherVariant =
            $crate::launcher::variant_type::LauncherVariant::$variant;
        const NAME: &'static str = const_str::convert_ascii_case!(snake, stringify!($variant));
        NAME
    }};
}

#[macro_export]
macro_rules! display_name {
    ($t:ty) => {{
        type _Check = $t;
        const NAME: &'static str = const_str::replace!(
            const_str::convert_ascii_case!(title, stringify!($t)),
            "_",
            " "
        );
        NAME
    }};
}
