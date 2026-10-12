use gpui::SharedString;

use crate::launcher::variant_type::QuickActionCapabilities;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum QuickAction {
    Hide,
}

impl std::fmt::Display for QuickAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hide => {
                write!(f, "Hide")
            }
        }
    }
}

impl From<QuickAction> for SharedString {
    fn from(value: QuickAction) -> Self {
        value.to_string().into()
    }
}

impl QuickAction {
    pub fn requires(&self) -> QuickActionCapabilities {
        match self {
            Self::Hide => QuickActionCapabilities::HIDE,
            _ => QuickActionCapabilities::empty(),
        }
    }
}

pub trait Hideable {
    fn hide_id(&self) -> Option<String> {
        None
    }
    fn is_hideable(&self) -> bool {
        false
    }
}
