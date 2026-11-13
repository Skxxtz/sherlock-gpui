use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::loader::utils::deserialize_path_buf;

mod alias;
mod config_impl;
mod defaults;
mod flags;
mod guard;
mod ignore;
mod imp;
mod keys;
mod reload;
mod sections;
mod transformer;
pub mod types;
mod watcher;
mod writer;

pub use alias::SherlockAlias;
pub use defaults::KeybindDefaults;
pub use flags::SherlockFlags;
pub use guard::ConfigGuard;
pub use ignore::SherlockIgnore;
pub use reload::{reload, reload_async};
pub use sections::*;
pub use transformer::repair_config;
pub use watcher::{ConfigFileChange, ConfigWatcher};
pub use writer::save_changes;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default)]
pub struct SherlockConfig {
    /// Whether the config was read from a file or uses defaults
    #[serde(skip)]
    pub initialized: bool,

    /// User-defined default applications (e.g., terminal, calendar)
    pub default_apps: ConfigDefaultApps,

    /// Preferred measurement units (e.g., length, temperature)
    pub units: ConfigUnits,

    /// Debugging preferences (e.g., whether to display errors)
    pub debug: ConfigDebug,
    /// UI preferences (e.g., window size, placeholder)
    pub appearance: ConfigAppearance,
    /// Runtime behavior settings (e.g., animations, clicks)
    pub behavior: ConfigBehavior,

    /// Custom key or action bindings (supplementing defaults)
    pub keybinds: ConfigKeybinds,

    /// User-specified overrides for default config file paths
    pub files: ConfigFiles,
    /// State from flags and piped input; read but never written
    #[serde(skip_serializing)]
    pub runtime: Runtime,

    /// Configures caching feature
    pub caching: ConfigCaching,

    /// Configures expand feature
    pub expand: ConfigExpand,

    /// Configures backdrop feature
    pub backdrop: ConfigBackdrop,

    /// Configures the status bar
    pub status_bar: StatusBar,

    /// Configures search bar icons
    pub search_bar_icon: SearchBarIcon,
}

impl SherlockConfig {
    /// Fixes values that make no sense; returns a warning for each.
    pub fn validate(&mut self) -> Vec<String> {
        let mut w = Vec::new();
        self.default_apps.validate(&mut w);
        self.units.validate(&mut w);
        self.debug.validate(&mut w);
        self.appearance.validate(&mut w);
        self.behavior.validate(&mut w);
        self.keybinds.validate(&mut w);
        self.files.validate(&mut w);
        self.runtime.validate(&mut w);
        self.caching.validate(&mut w);
        self.expand.validate(&mut w);
        self.backdrop.validate(&mut w);
        self.status_bar.validate(&mut w);
        self.search_bar_icon.validate(&mut w);
        w
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigSourceFiles {
    pub source: Vec<ConfigSource>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigSource {
    #[serde(deserialize_with = "deserialize_path_buf")]
    pub file: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::{types::*, *};

    /// A config as older versions wrote it still reads, and writes back in the same shape.
    #[test]
    fn reads_old_configs() {
        let old = r#"
        [default_apps]
        teams = "teams"
        calendar_client = "thunderbird"
        terminal = ""
        browser = "firefox %u"

        [units]
        lengths = "feet"
        weights = "kg"
        volumes = "l"
        temperatures = "F"
        currency = "USD"

        [appearance]
        width = 1000
        margins = [0, 0, 0, 0]
        anchor = ""
        opacity = 0.9

        [backdrop]
        enable = true
        opacity = 0.6
        edge = "top"

        [runtime]
        multi = true
        "#;
        let mut c: SherlockConfig = toml::from_str(old).unwrap();
        assert!(c.validate().is_empty());
        assert_eq!(c.appearance.width, 1000);
        assert_eq!(c.appearance.height, 593); // missing key: section default, not 0
        assert_eq!(c.appearance.anchor, Anchor::Center);
        assert_eq!(c.units.lengths, LengthUnit::Feet);
        assert_eq!(c.units.temperatures, TemperatureUnit::Fahrenheit);
        assert_eq!(&*c.units.currency, "usd");
        assert!(c.default_apps.terminal.is_none());
        assert_eq!(c.default_apps.browser().unwrap().as_ref(), "firefox %u");
        assert!(c.runtime.multi);

        let out = toml::to_string(&c).unwrap();
        assert!(out.contains("anchor = \"center\""), "{out}");
        assert!(out.contains("margins = [0, 0, 0, 0]"), "{out}");
        assert!(out.contains("opacity = 0.9\n"), "{out}");
        assert!(out.contains("temperatures = \"F\""), "{out}");
        assert!(!out.contains("[runtime]"), "{out}");
    }

    #[test]
    fn validate_fixes_nonsense() {
        let mut c: SherlockConfig =
            toml::from_str("[appearance]\nwidth = 0\nnum_shortcuts = 20\n").unwrap();
        assert_eq!(c.validate().len(), 2);
        assert_eq!(c.appearance.width, 900);
        assert_eq!(c.appearance.num_shortcuts, 9);
    }
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize, Default)]
pub enum HomeType {
    #[default]
    Search,
    OnlyHome,
    Home,
    Persist,
}
