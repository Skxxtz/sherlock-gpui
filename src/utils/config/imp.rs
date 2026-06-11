use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::utils::{
    config::{
        ConfigAppearance, ConfigBackdrop, ConfigBehavior, ConfigCaching, ConfigDefaultApps,
        ConfigExpand, ConfigFiles, ConfigKeybinds, ConfigUnits, KeybindDefaults, SearchBarIcon,
        StatusBar,
        defaults::{AppearanceDefaults, ConstantDefaults, FileDefaults, OtherDefaults},
    },
    files::home_dir,
};

impl Default for ConfigDefaultApps {
    fn default() -> Self {
        Self {
            teams: ConstantDefaults::teams(),
            calendar_client: ConstantDefaults::calendar_client(),
            terminal: ConstantDefaults::get_terminal().unwrap_or_default(), // Should never get to this...
            browser: ConstantDefaults::browser().ok(),
            mpris: None,
        }
    }
}

impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: ConstantDefaults::lengths(),
            weights: ConstantDefaults::weights(),
            volumes: ConstantDefaults::volumes(),
            temperatures: ConstantDefaults::temperatures(),
            currency: ConstantDefaults::currency(),
        }
    }
}

impl Default for ConfigAppearance {
    fn default() -> Self {
        Self {
            width: 900,
            height: 593, // 617 with, 593 without notification bar
            margins: (0, 0, 0, 0),
            anchor: String::from(""),
            icon_paths: FileDefaults::icon_paths(),
            icon_size: AppearanceDefaults::icon_size(),
            opacity: 1.0,
            mod_key_ascii: AppearanceDefaults::modkey_ascii(),
            num_shortcuts: 5,
            placeholder: AppearanceDefaults::placeholder(),
        }
    }
}

impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
            use_xdg_data_dir_icons: false,
            animate: true,
            global_prefix: None,
            global_flags: None,
            use_lr_nav: false,
            remember_query: false,
            n_clicks: Some(2),
        }
    }
}

impl Default for ConfigKeybinds {
    fn default() -> Self {
        KeybindDefaults::binds()
    }
}

impl Default for ConfigFiles {
    fn default() -> Self {
        Self {
            config: FileDefaults::config(),
            fallback: FileDefaults::fallback(),
            alias: FileDefaults::alias(),
            ignore: FileDefaults::ignore(),
            actions: FileDefaults::actions(),
        }
    }
}

impl Default for ConfigCaching {
    fn default() -> Self {
        Self {
            enable: true,
            cache: FileDefaults::cache(),
        }
    }
}

impl Default for ConfigExpand {
    fn default() -> Self {
        Self {
            enable: false,
            edge: OtherDefaults::backdrop_edge(),
            margin: 0,
        }
    }
}

impl Default for ConfigBackdrop {
    fn default() -> Self {
        Self {
            enable: false,
            opacity: OtherDefaults::backdrop_opacity(),
            edge: OtherDefaults::backdrop_edge(),
        }
    }
}

impl Default for SearchBarIcon {
    fn default() -> Self {
        Self {
            enable: true,
            icon: AppearanceDefaults::search_icon(),
            icon_back: AppearanceDefaults::search_icon_back(),
            icon_size: AppearanceDefaults::icon_size(),
            icon_back_size: AppearanceDefaults::icon_size(),
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self { enable: true }
    }
}

// With Root Implementations
pub trait WithRoot {
    fn with_root(root: &Path) -> Self;
}
impl WithRoot for ConfigAppearance {
    fn with_root(root: &Path) -> Self {
        let mut root = root.to_path_buf();
        if root.ends_with("/") {
            root.pop();
        }
        let root = root.to_str();
        fn use_root(root: Option<&str>, path: PathBuf) -> Option<PathBuf> {
            let root = root?;
            let home = home_dir().ok()?;
            let base = home.join(".config/sherlock");

            if let Ok(suffix) = path.strip_prefix(&base) {
                Some(Path::new(root).join(suffix))
            } else {
                None
            }
        }
        let icon_paths: Vec<PathBuf> = FileDefaults::icon_paths()
            .into_iter()
            .filter_map(|s| use_root(root, s))
            .collect();

        Self {
            icon_paths,
            ..Default::default()
        }
    }
}

impl WithRoot for ConfigFiles {
    fn with_root(root: &Path) -> Self {
        let mut root = root.to_path_buf();
        if root.ends_with("/") {
            root.pop();
        }
        fn use_root(root: &Path, path: Arc<Path>) -> Arc<Path> {
            if let Ok(stripped) = path.strip_prefix("~/.config/sherlock") {
                root.join(stripped).into()
            } else {
                path
            }
        }

        Self {
            config: use_root(&root, FileDefaults::config()),
            fallback: use_root(&root, FileDefaults::fallback()),
            alias: use_root(&root, FileDefaults::alias()),
            ignore: use_root(&root, FileDefaults::ignore()),
            actions: use_root(&root, FileDefaults::actions()),
        }
    }
}
