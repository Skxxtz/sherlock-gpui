use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use crate::{
    loader::utils::{deserialize_arc_path, deserialize_path_buf},
    ui::actions::UIFunction,
};

mod alias;
mod config_impl;
mod defaults;
mod flags;
mod guard;
mod ignore;
mod imp;
mod keys;
mod reload;
mod transformer;
mod watcher;
mod writer;

pub use alias::SherlockAlias;
pub use defaults::{
    AppearanceDefaults,
    ConstantDefaults,
    FileDefaults,
    KeybindDefaults,
    OtherDefaults,
};
pub use flags::SherlockFlags;
pub use guard::ConfigGuard;
pub use ignore::SherlockIgnore;
pub use reload::{reload, reload_async};
pub use transformer::repair_config;
pub use watcher::{ConfigFileChange, ConfigWatcher};
pub use writer::save_changes;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SherlockConfig {
    /// Whether the config was initialized or uses defaults
    #[serde(skip)]
    #[serde(default)]
    pub initialized: bool,

    /// User-defined default applications (e.g., terminal, calendar)
    #[serde(default)]
    pub default_apps: ConfigDefaultApps,

    /// Preferred measurement units (e.g., length, temperature)
    #[serde(default)]
    pub units: ConfigUnits,

    /// Debugging preferences (e.g., whether to display errors)
    #[serde(default)]
    pub debug: ConfigDebug,

    /// UI preferences (e.g., show/hide status bar)
    #[serde(default)]
    pub appearance: ConfigAppearance,

    /// Runtime behavior settings (e.g., daemon mode, caching)
    #[serde(default)]
    pub behavior: ConfigBehavior,

    /// Custom key or action bindings (supplementing defaults)
    #[serde(default)]
    pub keybinds: ConfigKeybinds,

    /// User-specified overrides for default config file paths
    #[serde(default)]
    pub files: ConfigFiles,

    /// Internal settings for JSON piping (e.g., default return action)
    #[serde(default)]
    pub runtime: Runtime,

    /// Configures caching feature
    #[serde(default)]
    pub caching: ConfigCaching,

    /// Configures expand feature
    #[serde(default)]
    pub expand: ConfigExpand,

    /// Configures backdrop feature
    #[serde(default)]
    pub backdrop: ConfigBackdrop,

    /// Configures the status bar
    #[serde(default)]
    pub status_bar: StatusBar,

    /// Configures search bar icons
    #[serde(default)]
    pub search_bar_icon: SearchBarIcon,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "default_apps", group = "defaults")]
pub struct ConfigDefaultApps {
    /// Opens Teams meetings.
    #[setting(label = "Teams")]
    #[serde(default = "ConstantDefaults::teams")]
    pub teams: String,
    /// Opens calendar events.
    #[setting(label = "Calendar")]
    #[serde(default = "ConstantDefaults::calendar_client")]
    pub calendar_client: String,
    /// Runs terminal commands.
    #[setting(label = "Terminal")]
    #[serde(default = "ConstantDefaults::terminal")]
    pub terminal: String,
    /// Opens links; empty uses the system default.
    #[setting(label = "Browser")]
    #[serde(default)]
    pub browser: Option<String>,
    /// The MPRIS player to control; empty picks the active one.
    #[setting(label = "Media player", group = "media")]
    #[serde(default)]
    pub mpris: Option<String>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigUnits {
    #[serde(default = "ConstantDefaults::lengths")]
    pub lengths: String,
    #[serde(default = "ConstantDefaults::weights")]
    pub weights: String,
    #[serde(default = "ConstantDefaults::volumes")]
    pub volumes: String,
    #[serde(default = "ConstantDefaults::temperatures")]
    pub temperatures: String,
    #[serde(default = "ConstantDefaults::currency")]
    pub currency: String,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone, Default)]
#[settings(path = "debug", group = "messages")]
pub struct ConfigDebug {
    /// Don't show error messages in the launcher.
    #[setting(label = "Hide errors")]
    #[serde(default)]
    pub try_suppress_errors: bool,
    /// Don't show warnings in the launcher.
    #[setting(label = "Hide warnings")]
    #[serde(default)]
    pub try_suppress_warnings: bool,
    #[serde(default)]
    pub app_paths: HashSet<String>,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "appearance", group = "window")]
pub struct ConfigAppearance {
    /// Window width in pixels.
    #[setting(label = "Width", range(300, 3000, 10))]
    #[serde(default)]
    pub width: i32,
    /// Window height in pixels.
    #[setting(label = "Height", range(200, 2000, 10))]
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub margins: (i32, i32, i32, i32),
    #[serde(default)]
    pub anchor: String,
    #[serde(default = "FileDefaults::icon_paths")]
    pub icon_paths: Vec<PathBuf>,
    /// Size of result icons in pixels.
    #[setting(label = "Icon size", group = "results", range(8, 64))]
    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_size: i32,
    /// Window opacity in percent.
    #[setting(label = "Opacity", range(10, 100, 5))]
    #[serde(default = "OtherDefaults::one")]
    pub opacity: f64,
    #[serde(default = "AppearanceDefaults::modkey_ascii")]
    pub mod_key_ascii: [char; 4],
    /// How many results get a numbered shortcut.
    #[setting(label = "Shortcut count", group = "results", range(0, 9))]
    #[serde(default = "OtherDefaults::five")]
    pub num_shortcuts: u8,
    /// Shown in the empty search bar.
    #[setting(label = "Placeholder", group = "search bar")]
    #[serde(default = "AppearanceDefaults::placeholder")]
    pub placeholder: SharedString,
}
#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "behavior", group = "navigation")]
pub struct ConfigBehavior {
    #[serde(default)]
    pub use_xdg_data_dir_icons: bool,
    /// Animate the window when it opens.
    #[setting(label = "Animate", group = "window")]
    #[serde(default = "OtherDefaults::bool_true")]
    pub animate: bool,
    /// Prepended to every command that is run.
    #[setting(label = "Global prefix", group = "search")]
    #[serde(default)]
    pub global_prefix: Option<String>,
    /// Appended to every command that is run.
    #[setting(label = "Global flags", group = "search")]
    #[serde(default)]
    pub global_flags: Option<String>,
    /// Move through grids and pages with ←→.
    #[setting(label = "Left/right navigation")]
    #[serde(default = "OtherDefaults::bool_true")]
    pub use_lr_nav: bool,
    /// How many clicks run an item; 0 uses the default.
    #[setting(label = "Clicks to run", range(0, 3))]
    #[serde(default)]
    pub n_clicks: Option<u8>,
    /// Keep the search text the next time Sherlock opens.
    #[setting(label = "Remember query", group = "search")]
    #[serde(default)]
    pub remember_query: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigFiles {
    #[serde(default = "FileDefaults::config")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub config: Arc<Path>,
    #[serde(default = "FileDefaults::fallback")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub fallback: Arc<Path>,
    #[serde(default = "FileDefaults::alias")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub alias: Arc<Path>,
    #[serde(default = "FileDefaults::ignore")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub ignore: Arc<Path>,
    #[serde(default = "FileDefaults::actions")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub actions: Arc<Path>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigKeybinds(pub HashMap<String, UIFunction>);

impl ConfigKeybinds {
    /// The user's keys in GPUI's spelling, and the ones that couldn't be read (with why).
    pub fn normalized(&self) -> (HashMap<String, UIFunction>, Vec<String>) {
        let mut binds = HashMap::new();
        let mut errors = Vec::new();
        for (key, function) in &self.0 {
            match keys::normalize(key) {
                Ok(key) => {
                    binds.insert(key, *function);
                }
                Err(e) => errors.push(format!(
                    "[keybinds] {e}: ignored, `{function:?}` keeps its default keys"
                )),
            }
        }
        (binds, errors)
    }

    /// What is bound, one action per key: the built-in keys, with the user's entries on top.
    /// A user key rebinds that key (also one of another action's built-in keys);
    /// `"key" = "unbind"` removes a built-in key.
    pub fn effective(&self) -> HashMap<String, UIFunction> {
        let (user, _) = self.normalized();
        let mut binds: HashMap<String, UIFunction> = KeybindDefaults::defaults()
            .iter()
            .map(|&(key, f)| (key.to_string(), f))
            .collect();
        for (key, function) in user {
            match function {
                UIFunction::Unbind => binds.remove(&key),
                function => binds.insert(key, function),
            };
        }
        binds
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Runtime {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub multi: bool,
    #[serde(default)]
    pub center: bool,
    #[serde(default)]
    pub photo_mode: bool,
    #[serde(default)]
    pub display_raw: bool,
    #[serde(default)]
    pub input: Option<bool>,
    #[serde(default)]
    pub sub_menu: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "caching", group = "performance")]
pub struct ConfigCaching {
    /// Keep a cache of installed apps for a faster start.
    #[setting(label = "Cache apps")]
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,
    #[serde(default = "FileDefaults::cache")]
    #[serde(deserialize_with = "deserialize_arc_path")]
    pub cache: Arc<Path>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigExpand {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "OtherDefaults::backdrop_edge")]
    pub edge: String,
    #[serde(default)]
    pub margin: i32,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "backdrop", group = "backdrop")]
pub struct ConfigBackdrop {
    /// Dim the screen behind the launcher.
    #[setting(label = "Backdrop")]
    #[serde(default)]
    pub enable: bool,
    /// How dark the backdrop is, in percent.
    #[setting(label = "Backdrop opacity", range(0, 100, 5))]
    #[serde(default = "OtherDefaults::backdrop_opacity")]
    pub opacity: f32,
    /// How long the backdrop fades in, in milliseconds; 0 uses the default.
    #[setting(label = "Fade duration", range(0, 1000, 50))]
    #[serde(default)]
    pub animation_duration: Option<u64>,
    #[serde(default = "OtherDefaults::backdrop_edge")]
    pub edge: String,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "search_bar_icon", group = "search bar")]
pub struct SearchBarIcon {
    /// Show the icon in the search bar.
    #[setting(label = "Search icon")]
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,

    /// The icon shown when the search bar is empty.
    #[setting(label = "Search icon")]
    #[serde(default = "AppearanceDefaults::search_icon")]
    pub icon: SharedString,

    /// The icon shown when the search bar is not empty.
    #[setting(label = "Back icon")]
    #[serde(default = "AppearanceDefaults::search_icon_back")]
    pub icon_back: SharedString,

    /// The icon size for the search icon.
    #[setting(label = "Icon size", range(0, 100, 1))]
    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_size: i32,

    /// The icon size for the back icon.
    #[setting(label = "Back icon size", range(0, 100, 1))]
    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_back_size: i32,
}

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "status_bar", group = "status bar")]
pub struct StatusBar {
    /// Show the bar at the bottom of the window.
    #[setting(label = "Status bar")]
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,
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

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize, Default)]
pub enum HomeType {
    #[default]
    Search,
    OnlyHome,
    Home,
    Persist,
}
