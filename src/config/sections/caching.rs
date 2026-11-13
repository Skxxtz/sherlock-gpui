use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;
use crate::config::{defaults::FileDefaults, types::CfgPath};

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "caching", group = "performance")]
#[serde(default)]
pub struct ConfigCaching {
    /// Keep a cache of installed apps for a faster start.
    #[setting(label = "Cache apps")]
    pub enable: bool,
    pub cache: CfgPath,
}

impl Default for ConfigCaching {
    fn default() -> Self {
        Self {
            enable: true,
            cache: FileDefaults::cache().into(),
        }
    }
}

impl Section for ConfigCaching {}
