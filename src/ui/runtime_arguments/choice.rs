use std::sync::Arc;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::choice::ChoiceOption;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceVar {
    pub choice: ChoiceVarBody,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceVarBody {
    pub name: SharedString,
    pub default: Option<SharedString>,
    pub choices: Arc<[ChoiceOption]>,
}
