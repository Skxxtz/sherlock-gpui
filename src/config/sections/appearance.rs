use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::{Section, check};
use crate::config::{
    defaults::FileDefaults,
    types::{Anchor, CfgPath, Margins, Percent, Text},
};

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "appearance", group = "window")]
#[serde(default)]
pub struct ConfigAppearance {
    /// Window width in pixels.
    #[setting(label = "Width", range(300, 3000, 10))]
    pub width: i32,
    /// Window height in pixels.
    #[setting(label = "Height", range(200, 2000, 10))]
    pub height: i32,
    /// Window margings in pixels.
    pub margins: Margins,
    /// Layershell anchor.
    pub anchor: Anchor,
    /// Additional icon paths added, apart from the default system paths.
    pub icon_paths: Vec<CfgPath>,
    /// Size of result icons in pixels.
    #[setting(label = "Icon size", group = "results", range(8, 64))]
    pub icon_size: i32,
    /// Window opacity in percent.
    #[setting(label = "Opacity", range(10, 100, 5))]
    pub opacity: Percent,
    /// The ascii characters that should be used for ctrl, super, alt, etc.
    pub mod_key_ascii: [char; 4],
    /// How many results get a numbered shortcut.
    #[setting(label = "Shortcut count", group = "results", range(0, 9))]
    pub num_shortcuts: u8,
    /// Shown in the empty search bar.
    #[setting(label = "Placeholder", group = "search bar")]
    pub placeholder: Text,
}

impl Default for ConfigAppearance {
    fn default() -> Self {
        Self {
            width: 900,
            height: 593, // 617 with, 593 without notification bar
            margins: Margins::default(),
            anchor: Anchor::Center,
            icon_paths: FileDefaults::icon_paths(),
            icon_size: 22,
            opacity: Percent::new(1.0),
            mod_key_ascii: ['⌘', '^', '⎇', '⇧'],
            num_shortcuts: 5,
            placeholder: Text::new_static("Search:"),
        }
    }
}

impl Section for ConfigAppearance {
    fn validate(&mut self, warn: &mut Vec<String>) {
        let d = Self::default();
        check(
            warn,
            "appearance.width",
            &mut self.width,
            |w| *w <= 0,
            d.width,
        );
        check(
            warn,
            "appearance.height",
            &mut self.height,
            |h| *h <= 0,
            d.height,
        );
        check(
            warn,
            "appearance.icon_size",
            &mut self.icon_size,
            |s| *s <= 0,
            d.icon_size,
        );
        check(
            warn,
            "appearance.num_shortcuts",
            &mut self.num_shortcuts,
            |n| *n > 9,
            9,
        );
    }
}
