use std::sync::Arc;

use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div,
    img, prelude::FluentBuilder, px,
};
use sherlock_derive::Hideable;

use crate::{
    app::theme::ThemeData,
    features::process::ProcessLauncherFunctions,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode, variant_type::InnerFunction},
    loader::{IconType, resolve_icon_path},
    ui::widgets::{
        RenderableChildImpl, Selection,
        process::{metadata::ProcessMeta, sidebar::ProcessSidebar},
    },
};

mod metadata;
mod sidebar;

#[derive(Clone, Default, Debug, Hideable)]
pub struct ProcessData {
    name: SharedString,
    pid: i32,
    ppid: i32,
    icon: Option<IconType>,
}

impl ProcessData {
    pub fn new(name: SharedString, pid: i32, ppid: i32) -> Self {
        Self {
            name,
            pid,
            ppid,
            icon: None,
        }
    }

    pub fn with_icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }
}

impl RenderableChildImpl for ProcessData {
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
            .child(if let Some(icon) = self.icon.as_ref() {
                if let Some(svg) = icon.svg() {
                    svg.size(px(24.))
                        .flex_shrink_0()
                        .text_color(theme.primary_text)
                        .into_any_element()
                } else {
                    img(icon.clone())
                        .size(px(24.))
                        .flex_shrink_0()
                        .into_any_element()
                }
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .min_w_0()
                    .w_full()
                    .child(
                        div()
                            .font_family(theme.font_family.clone())
                            .text_sm()
                            .w_full()
                            .text_color(theme.secondary_text)
                            .when(selection.is_selected, |this| {
                                this.text_color(theme.primary_text)
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme.font_family.clone())
                            .text_color(theme.secondary_text)
                            .child(if self.ppid == self.pid {
                                self.pid.to_string()
                            } else {
                                format!("{} > {}", self.ppid, self.pid)
                            }),
                    ),
            )
            .into_any_element()
    }

    #[inline(always)]
    fn sidebar(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<AnyElement> {
        let meta = ProcessMeta::new(self).ok()?;
        Some(ProcessSidebar::new(self, meta).into_any_element())
    }

    #[inline(always)]
    fn has_sidebar(&self) -> bool {
        true
    }

    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::Inner {
            func: InnerFunction::Process(ProcessLauncherFunctions::Quit { pid: self.pid }),
            exit: launcher.exit,
        })
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        Some(self.name.to_string())
    }

    #[inline(always)]
    fn search(&self, _launcher: &Arc<LauncherConfig>) -> &str {
        &self.name
    }
}
