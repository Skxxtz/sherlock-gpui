use std::sync::Arc;

use gpui::{AnyElement, App, Context, SharedString, Window};

use crate::{
    app::data::LauncherData,
    launcher::{LauncherConfig, variant_type::LauncherType},
    ui::{
        model::Model,
        traits::HandleActions,
        views::{
            Propagation,
            Query,
            QueryResult,
            ShellCtx,
            SherlockView,
            file::FileSearchModel,
            list::list_body,
            query::external_query,
        },
    },
};

pub struct FileView {
    pub model: Model,
    /// Fills `model` with results as the query changes.
    search: FileSearchModel,
}

impl FileView {
    pub fn new(launcher: Arc<LauncherConfig>, dir: Option<SharedString>, cx: &mut App) -> Self {
        let cap = match launcher.launcher_type.as_ref() {
            LauncherType::Files(p) => p.max_results,
            _ => 0,
        };
        Self {
            model: Model::external(&launcher, cap, cx),
            search: FileSearchModel::new(launcher, dir),
        }
    }
}

impl HandleActions for FileView {
    const UNHANDLED: Propagation = Propagation::Continue;
}

impl SherlockView for FileView {
    type Data = LauncherData;

    fn model(&self) -> Option<&Model<LauncherData>> {
        Some(&self.model)
    }

    fn model_mut(&mut self) -> Option<&mut Model<LauncherData>> {
        Some(&mut self.model)
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
        external_query(
            &self.model,
            query,
            cx,
            |v: &mut Self| &mut v.model,
            |q, data, sink, cx| self.search.search(q, data, sink, cx),
        )
    }
}
