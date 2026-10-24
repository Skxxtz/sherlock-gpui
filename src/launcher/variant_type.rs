use std::{mem, sync::Arc};

use gpui::{App, SharedString};
use serde::{Deserialize, Serialize};
use strum::{Display, IntoStaticStr};

use crate::{
    features::{
        app::AppLauncher,
        audio::{MusicPlayerFunctions, MusicPlayerLauncher},
        bookmark::BookmarkLauncher,
        calc::CalculatorLauncher,
        category::CategoryLauncher,
        clipboard::ClipboardLauncher,
        command::CommandLauncher,
        debug::{DebugFunctions, DebugLauncher},
        dmenu::DmenuLauncher,
        emoji::EmojiPicker,
        event::{EventLauncher, EventLauncherFunctions},
        file::FileLauncher,
        message::MessageLauncher,
        plugin::{PluginFunctions, PluginLauncher},
        process::{ProcessLauncher, ProcessLauncherFunctions},
        script::{ScriptFunctions, ScriptLauncher},
        theme::{ThemePicker, ThemePickerFunctions},
        timer::{TimerLauncher, TimerLauncherFunctions},
        translator::Translator,
        weather::WeatherLauncher,
        web::WebLauncher,
    },
    launcher::{Bind, ExecEffect, LauncherProvider},
    loader::utils::RawLauncher,
    ui::widgets::RenderableChild,
    utils::errors::SherlockMessage,
};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct QuickActionCapabilities: u8 {
        const HIDE = 1 << 0;
        const INFO = 1 << 1;
    }
}

macro_rules! create_variants {
    (@quick hideable) => { QuickActionCapabilities::HIDE };
    (@quick hasinfo) => { QuickActionCapabilities::INFO };
    (@quick $_:ident) => { QuickActionCapabilities::empty() };

    (@pickable pickable) => { true };
    (@pickable $_:ident) => { false };
    (
        enum $name:ident {
            $( $variant:ident( $inner:ty $(, $extra:ty)* )
               $(: $first:ident $(+ $rest:ident)*)? ),* $(,)?
        }
    ) => {

        #[derive(Clone, Debug, Default)]
        pub enum $name {
            $($variant($inner),)*
            #[default]
            Empty,
        }

        #[derive(Deserialize, Debug, Serialize, Clone, Copy, Default, Display, PartialEq, Eq, Hash, IntoStaticStr)]
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

        impl $name {
            pub const fn capabilities(&self) -> QuickActionCapabilities {
                match self {
                    Self::Empty => QuickActionCapabilities::empty(),
                    $(
                        Self::$variant(..) => QuickActionCapabilities::from_bits_retain(
                            0 $(
                                | create_variants!(@quick $first).bits()
                                $( | create_variants!(@quick $rest).bits() )*
                            )?
                        ),
                    )*
                }
            }
            pub const fn is_hideable(&self) -> bool { self.capabilities().contains(QuickActionCapabilities::HIDE) }

            pub fn is_pickable(&self) -> bool {
                self.variant().is_pickable()
            }

            pub fn variant(&self) -> LauncherVariant {
                LauncherVariant::from(self)
            }
            pub fn get_render_obj(
                &self,
                launcher: std::sync::Arc<crate::launcher::LauncherConfig>,
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
            pub fn try_into_launcher_type(self, raw: &RawLauncher) -> Result<$name, SherlockMessage> {
                match self {
                    $(
                        Self::$variant => <$inner as LauncherProvider>::try_parse(raw),
                    )*
                    Self::Empty => Ok($name::Empty)
                }
            }
            pub fn as_str(&self) -> &'static str {
                self.into()
            }
            pub const fn is_pickable(self) -> bool {
                match self {
                    $( Self::$variant => false
                        $( || create_variants!(@pickable $first)
                           $( || create_variants!(@pickable $rest) )* )?, )*
                    Self::Empty => false,
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

        #[cfg(feature = "docs")]
        mod docs {
            use super::*;
            use crate::docs::{Documentation, launcher::LauncherDoc};
            use md_rs::components::{ParentComponentExt, container::Container};

            //trait enforced
            #[allow(dead_code)]
            fn _assert_launcher_docs() {
                fn assert_launcher_doc<T: LauncherDoc>() {}
                $(
                    assert_launcher_doc::<$inner>();
                )*
            }

            impl Documentation for LauncherType {
                type Docs = Container;
                fn docs() -> Self::Docs {
                    Container::default().children(
                        [ $( <$inner>::doc(),)* ].iter().map(Container::from)
                    )
                }
            }

            #[cfg(test)]
            mod launcher_doc_tests {
                use super::*;
                use crate::docs::launcher::LauncherDocEntry;

                #[test]
                fn test_all_docs_valid() {
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
                            let launcher = var.try_into_launcher_type(&raw);
                            assert!(
                                launcher.is_ok(),
                                "{:?} example '{}' failed to parse: {:?}",
                                var,
                                example.description,
                                launcher.err(),
                            );
                        }
                    }
                }
            }
        }
    };
}

create_variants! {
    enum LauncherType {
        Apps(AppLauncher): hideable + hasinfo + pickable,
        Bookmarks(BookmarkLauncher): pickable,
        Calculator(CalculatorLauncher),
        Categories(CategoryLauncher): pickable,
        Clipboard(ClipboardLauncher),
        Commands(CommandLauncher): pickable,
        Debug(DebugLauncher, DebugFunctions),
        Dmenu(DmenuLauncher),
        Emoji(EmojiPicker): pickable,
        Event(EventLauncher, EventLauncherFunctions),
        Files(FileLauncher): hasinfo,
        Message(MessageLauncher),
        MusicPlayer(MusicPlayerLauncher, MusicPlayerFunctions),
        Plugin(PluginLauncher, PluginFunctions),
        Process(ProcessLauncher, ProcessLauncherFunctions),
        Script(ScriptLauncher, ScriptFunctions),
        Theme(ThemePicker, ThemePickerFunctions): pickable,
        Timer(TimerLauncher, TimerLauncherFunctions),
        Translator(Translator),
        Weather(WeatherLauncher),
        Web(WebLauncher): pickable,
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
