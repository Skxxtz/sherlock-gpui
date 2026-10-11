use gpui::SharedString;

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

pub trait Hideable {
    fn hide_id(&self) -> Option<String> {
        None
    }
    fn is_hideable(&self) -> bool {
        false
    }
}
