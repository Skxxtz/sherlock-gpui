pub mod backdrop;
pub mod components;
pub mod launcher;
pub mod model;
pub mod runtime_arguments;
pub mod traits;
mod utils;
pub mod widgets;

use gpui::KeyBinding;
use serde::{Deserialize, Serialize};
pub use utils::search::query_memory;

use crate::ui::{
    components::search_bar::actions::Complete,
    launcher::{
        Execute, NextVar, OpenContext, PrevVar, Quit, SelectionDown, SelectionLeft, SelectionUp,
    },
};

#[derive(Deserialize, Serialize, Hash, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UIFunction {
    Exit,

    ItemDown,
    ItemUp,
    ItemLeft,
    ItemRight,

    ArgNext,
    ArgPrev,

    Complete,

    Exec,
    ExecInplace,

    MultiSelect,

    ToggleContext,
    CloseContext,

    ClearBar,
    Backspace,

    ErrorPage,

    Shortcut,
}
impl UIFunction {
    pub fn get_binding(&self, key: &str) -> Option<KeyBinding> {
        // split off namespace
        let mut parts = key.rsplitn(2, '.');

        // rsplitn gives us the right-most part first (the key)
        let key_part = parts.next().unwrap_or("");
        let scope = parts.next();

        match self {
            Self::Exit => Some(KeyBinding::new(key_part, Quit, scope)),
            Self::ItemDown => Some(KeyBinding::new(key_part, SelectionDown, scope)),
            Self::ItemUp => Some(KeyBinding::new(key_part, SelectionUp, scope)),
            Self::ItemLeft => Some(KeyBinding::new(key_part, SelectionLeft, scope)),
            Self::ItemRight => Some(KeyBinding::new(key_part, SelectionLeft, scope)),
            Self::Exec => Some(KeyBinding::new(key_part, Execute, scope)),
            Self::ArgNext => Some(KeyBinding::new(key_part, NextVar, scope)),
            Self::ArgPrev => Some(KeyBinding::new(key_part, PrevVar, scope)),
            Self::ToggleContext => Some(KeyBinding::new(key_part, OpenContext, scope)),
            Self::Complete => Some(KeyBinding::new(key_part, Complete, scope)),

            _ => None,
        }
    }
}
