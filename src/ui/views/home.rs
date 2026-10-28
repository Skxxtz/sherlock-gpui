use gpui::{AnyElement, App, Context, Window, px};

use crate::{
    app::{LauncherEntity, data::LauncherData},
    ui::{
        launcher::navigator::views::Layout,
        model::Model,
        traits::HandleActions,
        views::{
            Propagation,
            Query,
            QueryResult,
            ShellCtx,
            SherlockView,
            list::list_body,
            query::launcher_query,
        },
    },
};

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

impl HandleActions for HomeView {
    const UNHANDLED: Propagation = Propagation::Continue;
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
