use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "status_bar", group = "status bar")]
#[serde(default)]
pub struct StatusBar {
    /// Show the bar at the bottom of the window.
    #[setting(label = "Status bar")]
    pub enable: bool,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self { enable: true }
    }
}

impl Section for StatusBar {}
