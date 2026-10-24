use std::{collections::HashMap, sync::Arc};

use gpui::App;

use crate::{
    features::emoji::{EMOJIS, EmojiData},
    launcher::{Launcher, LauncherConfig},
    ui::{model::Model, widgets::RenderableChild},
};

pub struct EmojiView {
    pub model: Model,
}

impl EmojiView {
    pub fn new(config: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let data: Vec<RenderableChild> = EMOJIS
            .iter()
            .map(|entry| RenderableChild::new(config.clone(), EmojiData { entry }))
            .collect();

        let launchers = HashMap::from([(
            config.id(),
            Launcher {
                config,
                children: data,
            },
        )]);

        Self {
            model: Model::standard(launchers, cx),
        }
    }
}
