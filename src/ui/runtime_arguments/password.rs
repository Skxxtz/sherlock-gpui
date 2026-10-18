use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct PasswordVar {
    pub password_input: SharedString,
    pub default: Option<SharedString>,
}
