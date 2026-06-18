use std::sync::Arc;

use gpui::App;

use crate::{
    launcher::{
        Launcher, LauncherConfig,
        emoji_launcher::{EmojiData, data::EMOJIS},
    },
    ui::{model::Model, widgets::RenderableChild},
};

pub struct EmojiView {
    pub model: Model,
}

impl EmojiView {
    pub fn new(config: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let data: Vec<RenderableChild> = EMOJIS
            .iter()
            .map(|entry| RenderableChild::Emoji {
                launcher: config.clone(),
                inner: EmojiData { entry },
            })
            .collect();

        let launcher_vec = vec![Launcher {
            config,
            children: data,
        }];

        Self {
            model: Model::standard(launcher_vec, cx),
        }
    }
}
