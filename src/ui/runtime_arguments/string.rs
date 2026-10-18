use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

use crate::ui::utils::input_rule::CharSet;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct StringVar {
    pub string_input: SharedString,
    pub default: Option<SharedString>,
    pub validation: Option<CharSet>,
}
