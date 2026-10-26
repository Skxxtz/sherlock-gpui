use crate::ui::views::{
    Chrome, ShellCtx, SherlockView,
    list::{grid_body, list_body},
};
use gpui::{AnyElement, Context, Window};
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
                children: data.into(),
            },
        )]);

        Self {
            model: Model::standard(launchers, cx),
        }
    }
}

impl SherlockView for EmojiView {
    fn chrome(&self) -> Chrome {
        Chrome::NO_SIDEBAR
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &shell.layout.grid {
            Some(grid) => grid_body(&self.model, grid, shell),
            None => list_body(&self.model, shell, cx),
        }
    }
}
