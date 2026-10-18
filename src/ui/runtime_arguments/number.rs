use crate::ui::utils::input_rule::CharSet;
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct NumberVar {
    pub number_input: SharedString,
    pub default: Option<f64>,
    pub validation: Option<CharSet>,
}
