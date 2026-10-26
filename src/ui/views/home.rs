use crate::app::data::LauncherData;
use crate::ui::launcher::navigator::views::Layout;
use crate::ui::views::{Query, QueryResult, query::launcher_query};
use crate::ui::views::{ShellCtx, SherlockView, list::list_body};
use gpui::App;
use gpui::px;
use gpui::{AnyElement, Context, Window};

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

impl SherlockView for HomeView {
    type Data = LauncherData;

    fn model(&self) -> Option<&Model<LauncherData>> {
        Some(&self.model)
    }

    fn model_mut(&mut self) -> Option<&mut Model<LauncherData>> {
        Some(&mut self.model)
    }

    fn layout(&self) -> Layout {
        Layout::row(self.model.len(), px(48.))
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        list_body(&self.model, shell, cx)
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        launcher_query(&mut self.model, query, cx, |v: &mut Self| &mut v.model)
    }
}
