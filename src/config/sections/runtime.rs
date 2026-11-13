use serde::{Deserialize, Serialize};

use super::Section;

/// State from the command line flags and piped input; not written to the config file.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Runtime {
    pub method: Option<String>,
    pub multi: bool,
    pub center: bool,
    pub photo_mode: bool,
    pub display_raw: bool,
    pub input: Option<bool>,
    pub sub_menu: Option<String>,
    pub field: Option<String>,
}

impl Section for Runtime {}
