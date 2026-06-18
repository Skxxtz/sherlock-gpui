use std::sync::Arc;

use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, Styled, div, img,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{LauncherConfig, app_launcher::app_data::AppData, utils::exec_mode::ExecMode},
    loader::utils::Priority,
    ui::{
        launcher::context_menu::ContextMenuAction,
        utils::{render::substitute, selection::Selection},
        widgets::RenderableChildImpl,
    },
    utils::config::ConfigGuard,
};

impl<'a> RenderableChildImpl<'a> for AppData {
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: Selection,
        query: &str,
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
            .child(if let Some(icon) = self.icon.as_ref() {
                if let Some(svg) = icon.svg() {
                    svg.size(px(24.))
                        .text_color(theme.primary_text)
                        .into_any_element()
                } else {
                    img(icon.clone()).size(px(24.)).into_any_element()
                }
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(
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
                            .when_some(self.name.as_ref(), |this, name| {
                                this.child(substitute(name.clone(), "keyword", query))
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme.font_family.clone())
                            .text_color(theme.secondary_text)
                            .when_some(launcher.name.as_ref(), |this, name| {
                                this.child(name.clone())
                            }),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::from_appdata(self, launcher))
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        let config = ConfigGuard::read().ok();
        let field = config.as_ref().and_then(|c| c.runtime.field.as_deref());
        match field {
            Some("exec") => self.exec.clone(),
            _ => self.name.as_ref().map(|n| n.to_string()),
        }
    }
    #[inline(always)]
    fn priority(&self, _launcher: &Arc<LauncherConfig>) -> Priority {
        self.priority.get()
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.search_string
    }
    #[inline(always)]
    fn actions(
        &self,
        _launcher: &Arc<LauncherConfig>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        Some(self.actions.clone())
    }
    #[inline(always)]
    fn has_actions(&self, _cx: &mut App) -> bool {
        !self.actions.is_empty()
    }
    #[inline(always)]
    fn vars(&self, _cx: &mut App) -> Option<&[crate::loader::utils::ExecVariable]> {
        Some(&self.vars) // Works for Vec or SmallVec
    }
    #[inline(always)]
    fn increment_count(&self) {
        self.priority.increment_count()
    }
}
