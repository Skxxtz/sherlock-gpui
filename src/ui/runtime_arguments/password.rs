use gpui::SharedString;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PasswordVar {
    pub password_input: SharedString,
    pub default: Option<SharedString>,
}
