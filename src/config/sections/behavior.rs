use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;
use crate::config::types::{Text, non_empty};

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "behavior", group = "navigation")]
#[serde(default)]
pub struct ConfigBehavior {
    pub use_xdg_data_dir_icons: bool,
    /// Animate the window when it opens.
    #[setting(label = "Animate", group = "window")]
    pub animate: bool,
    /// Prepended to every command that is run.
    #[setting(label = "Global prefix", group = "search")]
    #[serde(deserialize_with = "non_empty")]
    pub global_prefix: Option<Text>,
    /// Appended to every command that is run.
    #[setting(label = "Global flags", group = "search")]
    #[serde(deserialize_with = "non_empty")]
    pub global_flags: Option<Text>,
    /// Move through grids and pages with ←→.
    #[setting(label = "Left/right navigation")]
    pub use_lr_nav: bool,
    /// How many clicks run an item; 0 uses the default.
    #[setting(label = "Clicks to run", range(0, 3))]
    pub n_clicks: Option<u8>,
    /// Keeps Sherlock's last state, including the search bar text, and last used views.
    #[setting(label = "Remember last state.", group = "window")]
    pub keep_state: bool,
}

impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
            use_xdg_data_dir_icons: false,
            animate: true,
            global_prefix: None,
            global_flags: None,
            use_lr_nav: true,
            n_clicks: None,
            keep_state: false,
        }
    }
}

impl Section for ConfigBehavior {
    fn validate(&mut self, _: &mut Vec<String>) {
        if self.n_clicks == Some(0) {
            self.n_clicks = None;
        }
    }
}
