use gpui::SharedString;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathVar {
    pub path_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
}
