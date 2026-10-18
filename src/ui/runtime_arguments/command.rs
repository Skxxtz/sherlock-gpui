use gpui::SharedString;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandVar {
    pub command_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
    #[serde(skip)]
    pub is_scoped: bool,
}

impl From<SharedString> for CommandVar {
    fn from(value: SharedString) -> Self {
        Self {
            command_input: value,
            ..Default::default()
        }
    }
}
