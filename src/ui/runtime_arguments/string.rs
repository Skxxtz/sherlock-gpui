use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::utils::input_rule::CharSet;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StringVar {
    pub string_input: SharedString,
    pub default: Option<SharedString>,
    pub validation: Option<CharSet>,
}
