use serde::{Deserialize, Serialize};
use sherlock_derive::Settings;

use super::Section;
use crate::{
    config::{
        defaults::ConstantDefaults,
        types::{Text, non_empty},
    },
    utils::errors::SherlockMessage,
};

#[derive(Settings, Deserialize, Serialize, Debug, Clone)]
#[settings(path = "default_apps", group = "defaults")]
#[serde(default)]
pub struct ConfigDefaultApps {
    /// Opens Teams meetings.
    #[setting(label = "Teams")]
    pub teams: Text,
    /// Opens calendar events.
    #[setting(label = "Calendar")]
    pub calendar_client: Text,
    /// Runs terminal commands; empty detects one.
    #[setting(label = "Terminal")]
    #[serde(deserialize_with = "non_empty")]
    pub terminal: Option<Text>,
    /// Opens links; empty uses the system default.
    #[setting(label = "Browser")]
    #[serde(deserialize_with = "non_empty")]
    pub browser: Option<Text>,
    /// The MPRIS player to control; empty picks the active one.
    #[setting(label = "Media player", group = "media")]
    #[serde(deserialize_with = "non_empty")]
    pub mpris: Option<Text>,
}

impl Default for ConfigDefaultApps {
    fn default() -> Self {
        Self {
            teams: Text::new_static(
                "teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}",
            ),
            calendar_client: Text::new_static("thunderbird"),
            terminal: None,
            browser: None,
            mpris: None,
        }
    }
}

impl Section for ConfigDefaultApps {}

impl ConfigDefaultApps {
    /// The configured terminal, or a detected one (looked up once). Empty if none is found.
    pub fn terminal(&self) -> Text {
        self.terminal
            .clone()
            .or_else(ConstantDefaults::detected_terminal)
            .unwrap_or_default()
    }

    /// The configured browser, or the system default (looked up once it's found).
    pub fn browser(&self) -> Result<Text, SherlockMessage> {
        match &self.browser {
            Some(b) => Ok(b.clone()),
            None => ConstantDefaults::browser(),
        }
    }
}
