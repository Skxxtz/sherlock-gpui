use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;
use crate::config::types::CfgPath;

#[derive(Settings, Deserialize, Serialize, Debug, Clone, Default)]
#[settings(path = "debug", group = "messages")]
#[serde(default)]
pub struct ConfigDebug {
    /// Don't show error messages in the launcher.
    #[setting(label = "Hide errors")]
    pub try_suppress_errors: bool,
    /// Don't show warnings in the launcher.
    #[setting(label = "Hide warnings")]
    pub try_suppress_warnings: bool,
    /// Extra directories to look for `.desktop` files in.
    pub app_paths: HashSet<CfgPath>,
}

impl Section for ConfigDebug {}
