use std::sync::Arc;

use gpui::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, div, prelude::FluentBuilder,
};

use crate::{
    app::theme::ThemeData,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::utils::Priority,
    ui::widgets::{RenderableChildImpl, Selection},
};

#[derive(Clone, Default)]
pub struct DmenuData {
    name: SharedString,
}

impl<'a> RenderableChildImpl<'a> for DmenuData {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(
                div().flex_col().justify_between().items_center().child(
                    div()
                        .text_sm()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(self.name.clone()),
                ),
            )
            .into_any_element()
    }
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        self.get_content(launcher, cx)
            .map(|content| ExecMode::Print { content })
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        Some(self.name.to_string())
    }
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.name
    }
}

impl<T> From<T> for DmenuData
where
    T: Into<SharedString>,
{
    fn from(value: T) -> Self {
        Self { name: value.into() }
    }
}
