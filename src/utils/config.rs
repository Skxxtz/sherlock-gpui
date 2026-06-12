use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    loader::utils::{deserialize_arc_path, deserialize_path_buf},
    ui::UIFunction,
};

mod config_impl;
mod defaults;
mod flags;
mod guard;
mod imp;
mod reload;
mod transformer;
mod watcher;

pub use defaults::{
    AppearanceDefaults, ConstantDefaults, FileDefaults, KeybindDefaults, OtherDefaults,
};
pub use flags::SherlockFlags;
pub use guard::ConfigGuard;
pub use reload::reload;
pub use transformer::repair_config;
pub use watcher::{ConfigFileChange, ConfigWatcher};

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
    #[serde(default = "KeybindDefaults::binds")]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigDefaultApps {
    #[serde(default = "ConstantDefaults::teams")]
    pub teams: String,
    #[serde(default = "ConstantDefaults::calendar_client")]
    pub calendar_client: String,
    #[serde(default = "ConstantDefaults::terminal")]
    pub terminal: String,
    #[serde(default)]
    pub browser: Option<String>,
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ConfigDebug {
    #[serde(default)]
    pub try_suppress_errors: bool,
    #[serde(default)]
    pub try_suppress_warnings: bool,
    #[serde(default)]
    pub app_paths: HashSet<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigAppearance {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub margins: (i32, i32, i32, i32),
    #[serde(default)]
    pub anchor: String,
    #[serde(default = "FileDefaults::icon_paths")]
    pub icon_paths: Vec<PathBuf>,
    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_size: i32,
    #[serde(default = "OtherDefaults::one")]
    pub opacity: f64,
    #[serde(default = "AppearanceDefaults::modkey_ascii")]
    pub mod_key_ascii: [char; 4],
    #[serde(default = "OtherDefaults::five")]
    pub num_shortcuts: u8,
    #[serde(default = "AppearanceDefaults::placeholder")]
    pub placeholder: SharedString,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBehavior {
    #[serde(default)]
    pub use_xdg_data_dir_icons: bool,
    #[serde(default = "OtherDefaults::bool_true")]
    pub animate: bool,
    #[serde(default)]
    pub global_prefix: Option<String>,
    #[serde(default)]
    pub global_flags: Option<String>,
    #[serde(default = "OtherDefaults::bool_true")]
    pub use_lr_nav: bool,
    #[serde(default)]
    pub n_clicks: Option<u8>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigKeybinds(pub HashMap<String, UIFunction>);

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigCaching {
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBackdrop {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "OtherDefaults::backdrop_opacity")]
    pub opacity: f64,
    #[serde(default = "OtherDefaults::backdrop_edge")]
    pub edge: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchBarIcon {
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,

    #[serde(default = "AppearanceDefaults::search_icon")]
    pub icon: SharedString,

    #[serde(default = "AppearanceDefaults::search_icon_back")]
    pub icon_back: SharedString,

    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_size: i32,

    #[serde(default = "AppearanceDefaults::icon_size")]
    pub icon_back_size: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatusBar {
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
