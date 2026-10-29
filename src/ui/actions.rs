#[macro_export]
macro_rules! sherlock_actions {
    (
        $(
            $module:ident $(as $namespace:ident)? : [ $( $(#[$attr:meta])* $name:ident ),* $(,)? ]
            $({ $($extra:item)* })?
        ),+ $(,)? ;
        special: [ $( $(#[$sattr:meta])* $special:ident ),* $(,)? ] $(,)?
    ) => {
        $(
            pub mod $module {
                $crate::sherlock_actions!(@structs $($namespace)? $module; $( $(#[$attr])* $name ),*);
                $( $($extra)* )?
            }
        )+

        /// A bindable action, as named in `[keybinds]` (`"item_down"`).
        #[derive(::std::clone::Clone, ::std::marker::Copy, ::std::fmt::Debug,
         ::std::cmp::PartialEq, ::std::cmp::Eq, ::std::hash::Hash,
         ::std::cmp::PartialOrd, ::std::cmp::Ord,
         ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum UIFunction {
            $( $( $name, )* )+
            $( $(#[$sattr])* $special, )*
        }

        impl UIFunction {
            /// The binding for a plain action; `None` for the special ones (bound by hand).
            pub fn binding(
                self,
                keys: &str,
                context: ::std::option::Option<&str>,
            ) -> ::std::option::Option<::gpui::KeyBinding> {
                match self {
                    $( $( Self::$name => Some(::gpui::KeyBinding::new(keys, $module::$name, context)), )* )+
                    $( Self::$special => None, )*
                }
            }
        }

        $( $(
            impl ::std::convert::From<&$module::$name> for UIFunction {
                fn from(_: &$module::$name) -> Self {
                    Self::$name
                }
            }
        )* )+
    };

    // The structs of one module. `#[action(namespace = …)]` needs a plain ident (attribute
    // arguments aren't expanded), so the namespace arrives here already chosen.
    (@structs $namespace:ident $module:ident; $( $(#[$attr:meta])* $name:ident ),*) => {
        $(
            #[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::default::Default,
                     ::std::fmt::Debug, ::gpui::Action)]
            #[action(namespace = $namespace)]
            $(#[$attr])*
            pub struct $name;
        )*
    };
    // no `as`: the namespace is the module's name
    (@structs $module:ident; $( $(#[$attr:meta])* $name:ident ),*) => {
        $crate::sherlock_actions!(@structs $module $module; $( $(#[$attr])* $name ),*);
    };
}

sherlock_actions! {
    launcher: [
        Quit, ItemUp, ItemDown, ItemLeft, ItemRight, NextVar, PrevVar,
        Execute, ExecuteInplace, ToggleContext, Back, ErrorPage,
    ],
    search_bar as text_input: [
        Backspace, Complete, DeleteForward, DeleteAll, CursorLeft, CursorRight,
        SelectLeft, SelectRight, SelectAll, Home, End, ShowCharacterPalette, Paste, Cut, Copy,
    ] {
        /// Runs the n-th result (`ctrl-<digit>`). Bound by hand (`UIFunction::Shortcut`), never
        /// built from JSON, so `no_json` (a struct with fields would need `JsonSchema` otherwise).
        #[derive(Clone, PartialEq, Default, Debug, ::gpui::Action)]
        #[action(namespace = text_input, no_json)]
        pub struct ShortcutAction {
            pub index: usize,
        }
    };
    special: [Shortcut, #[serde(alias = "none")] Unbind],
}

pub use launcher::*;
pub use search_bar::*;
