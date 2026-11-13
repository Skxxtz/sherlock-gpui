use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;
use crate::config::types::{Edge, Percent};

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "backdrop", group = "backdrop")]
#[serde(default)]
pub struct ConfigBackdrop {
    /// Dim the screen behind the launcher.
    #[setting(label = "Backdrop")]
    pub enable: bool,
    /// How dark the backdrop is, in percent.
    #[setting(label = "Backdrop opacity", range(0, 100, 5))]
    pub opacity: Percent,
    /// How long the backdrop fades in, in milliseconds; 0 uses the default.
    #[setting(label = "Fade duration", range(0, 1000, 50))]
    pub animation_duration: Option<u64>,
    pub edge: Edge,
}

impl Default for ConfigBackdrop {
    fn default() -> Self {
        Self {
            enable: false,
            opacity: Percent::new(0.6),
            animation_duration: None,
            edge: Edge::Top,
        }
    }
}

impl Section for ConfigBackdrop {}
