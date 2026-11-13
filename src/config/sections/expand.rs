use serde::{Deserialize, Serialize};

use super::Section;
use crate::config::types::Edge;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ConfigExpand {
    pub enable: bool,
    pub edge: Edge,
    pub margin: i32,
}

impl Section for ConfigExpand {}
