use std::borrow::Cow;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::{
    ui::runtime_arguments::utils::{Finalize, expand_one},
    utils::files::home_str,
};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathVar {
    pub path_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
}

impl Finalize for PathVar {
    fn finalize<'a>(&self, raw: &'a str) -> Cow<'a, str> {
        expand_one(raw, home_str().unwrap_or("."))
    }
}
