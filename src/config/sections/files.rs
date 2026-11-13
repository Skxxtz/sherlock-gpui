use serde::{Deserialize, Serialize};

use super::Section;
use crate::config::{defaults::FileDefaults, types::CfgPath};

/// Where Sherlock's other files are.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct ConfigFiles {
    pub config: CfgPath,
    pub fallback: CfgPath,
    pub alias: CfgPath,
    pub ignore: CfgPath,
    pub actions: CfgPath,
}

impl Default for ConfigFiles {
    fn default() -> Self {
        Self {
            config: FileDefaults::config().into(),
            fallback: FileDefaults::fallback().into(),
            alias: FileDefaults::alias().into(),
            ignore: FileDefaults::ignore().into(),
            actions: FileDefaults::actions().into(),
        }
    }
}

impl Section for ConfigFiles {}
