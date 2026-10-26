use std::sync::Arc;

use gpui::{AnyElement, App, Context, Window};

use crate::{
    launcher::{LauncherConfig, variant_type::LauncherType},
    ui::{
        model::Model,
        views::{
            Query, QueryResult, ShellCtx, SherlockView, list::list_body, process::ProcessModel,
            query::external_query,
        },
    },
};

pub struct ProcessView {
    pub model: Model,
    /// Fills `model` with results as the query changes.
    search: ProcessModel,
}

impl ProcessView {
    pub fn new(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let cap = match launcher.launcher_type.as_ref() {
            LauncherType::Process(p) => p.max_results,
            _ => 0,
        };
        Self {
            model: Model::external(&launcher, cap, cx),
            search: ProcessModel::new(launcher),
        }
    }
}

impl SherlockView for ProcessView {
    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        list_body(&self.model, shell, cx)
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        external_query(&self.model, query, cx, |q, data, shell, cx| {
            self.search.search(q, data, shell, cx)
        })
    }
}
