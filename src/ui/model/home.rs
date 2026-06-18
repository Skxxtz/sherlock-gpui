use gpui::App;

use crate::{app::LauncherEntity, ui::model::Model};

pub struct HomeView {
    pub model: Model,
}

impl HomeView {
    pub fn new(entity: LauncherEntity, cx: &mut App) -> Self {
        Self {
            model: Model::standard_with_entity(entity, cx),
        }
    }
}
