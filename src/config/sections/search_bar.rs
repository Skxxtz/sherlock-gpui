use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::{Section, check};
use crate::config::types::Text;

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "search_bar_icon", group = "search bar")]
#[serde(default)]
pub struct SearchBarIcon {
    /// Show the icon in the search bar.
    #[setting(label = "Search icon")]
    pub enable: bool,
    /// The icon shown when the search bar is empty.
    #[setting(label = "Search icon")]
    pub icon: Text,
    /// The icon shown when the search bar is not empty.
    #[setting(label = "Back icon")]
    pub icon_back: Text,
    /// The icon size for the search icon.
    #[setting(label = "Icon size", range(0, 100, 1))]
    pub icon_size: i32,
    /// The icon size for the back icon.
    #[setting(label = "Back icon size", range(0, 100, 1))]
    pub icon_back_size: i32,
}

impl Default for SearchBarIcon {
    fn default() -> Self {
        Self {
            enable: true,
            icon: Text::new_static("sherlock-search"),
            icon_back: Text::new_static("sherlock-back"),
            icon_size: 22,
            icon_back_size: 22,
        }
    }
}

impl Section for SearchBarIcon {
    fn validate(&mut self, warn: &mut Vec<String>) {
        let d = Self::default();
        check(
            warn,
            "search_bar_icon.icon_size",
            &mut self.icon_size,
            |s| *s < 0,
            d.icon_size,
        );
        check(
            warn,
            "search_bar_icon.icon_back_size",
            &mut self.icon_back_size,
            |s| *s < 0,
            d.icon_back_size,
        );
    }
}
