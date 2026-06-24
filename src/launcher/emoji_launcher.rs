use std::{fmt::Display, sync::Arc};

use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::{
    launcher::{
        LauncherConfig, LauncherProvider, LauncherType, app_launcher::app_data::AppData,
        emoji_launcher::data::EmojiEntry,
    },
    loader::{
        resolve_icon_path,
        utils::{PriorityGuard, RawLauncher},
    },
    ui::widgets::{RenderableChild, emoji::set_selected_skin_tone},
    utils::errors::SherlockMessage,
};

pub mod data;

pub static ALL_SKIN_TONES: [SkinTone; 6] = [
    SkinTone::Simpsons,
    SkinTone::Light,
    SkinTone::MediumLight,
    SkinTone::Medium,
    SkinTone::MediumDark,
    SkinTone::Dark,
];

/// The following arguments are available to users:
/// - `default_skin_tone`: The skin tone that should be used as default
#[derive(Clone, Debug, Default)]
pub struct EmojiPicker {}

impl LauncherProvider for EmojiPicker {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Emoji(Self {}))
    }

    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let inner = AppData {
            name: launcher.name.as_ref().map(Into::into),
            search_string: "emoji".into(),
            icon: resolve_icon_path("sherlock-emoji"),
            priority: PriorityGuard::new_with_launcher(&launcher, 0),
            ..AppData::new()
        };

        let default_skin_tone: SkinTone = opts
            .get("default_skin_tone")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or(SkinTone::Simpsons);
        set_selected_skin_tone(default_skin_tone, 0);

        let child = RenderableChild::App { launcher, inner };

        Ok(vec![child])
    }
}

#[derive(Clone, Debug)]
pub struct EmojiData {
    pub entry: &'static EmojiEntry,
}

#[derive(Copy, Clone, Debug, FromRepr, Default, Deserialize, Serialize, PartialEq)]
#[repr(u8)] // This tells Rust to treat the enum like a u8 in memory
pub enum SkinTone {
    #[default]
    Simpsons = 0,
    Light = 1,
    MediumLight = 2,
    Medium = 3,
    MediumDark = 4,
    Dark = 5,
}

impl Display for SkinTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
            Self::Simpsons => "",
        };
        f.write_str(s)
    }
}
impl From<u8> for SkinTone {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Light,
            2 => Self::MediumLight,
            3 => Self::Medium,
            4 => Self::MediumDark,
            5 => Self::Dark,
            _ => Self::Simpsons,
        }
    }
}

impl SkinTone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
            Self::Simpsons => "",
        }
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::EmojiPicker;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };
    use indoc::indoc;

    impl LauncherDoc for EmojiPicker {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(EmojiPicker),
                variant_name: variant_name!(Emoji),
                description: "A emoji picker allowing for skin tone selection.",
                args: &[FieldDoc {
                    name: "default_skin_tone",
                    ty: "SkinTone",
                    required: false,
                    default: Some("Simpsons"),
                    description: "The skin tone to use as the default. Can be either: Light, MediumLight, Medium, MediumDark, Dark, or Simpsons",
                }],
                examples: &[Example {
                    description: "Basic emoji picker",
                    json: indoc! {
                        r#"{
                        "name": "Emoji Picker",
                        "alias": "emj",
                        "type": "emoji",
                        "args": {
                            "default_skin_tone": "Simpsons"
                        },
                        "priority": 5,
                        "home": "Home"
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}
