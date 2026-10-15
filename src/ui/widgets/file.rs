use crate::{
    app::theme::ThemeData,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::{IconType, resolve_icon_path, utils::Priority},
    ui::{
        launcher::views::NavigationViewType,
        traits::RenderableChildImpl,
        utils::selection::Selection,
        widgets::file::{metadata::FileMeta, sidebar::FileSidebar},
    },
};
use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div,
    img, prelude::FluentBuilder, px,
};
use sherlock_derive::Hideable;
use std::sync::Arc;

mod metadata;
pub mod sidebar;

#[derive(Clone, Default, Debug, Hideable)]
pub struct FileData {
    loc: SharedString,
    name: SharedString,
    icon: Option<IconType>,
}

impl FileData {
    pub fn new(loc: Arc<str>) -> Self {
        let name: Arc<str> = loc
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(_, name)| name)
            .unwrap_or(&loc)
            .into();
        Self {
            loc: loc.clone().into(),
            name: name.into(),
            icon: None,
        }
    }

    pub fn with_icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }
}
impl<'a> RenderableChildImpl<'a> for FileData {
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
                            .font_family(theme.font_family.clone())
                            .text_xs()
                            .w_full()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_color(theme.secondary_text)
                            .child(self.loc.clone()),
                    ),
            )
            .into_any_element()
    }

    #[inline(always)]
    fn sidebar(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<AnyElement> {
        let meta = FileMeta::new(self)?;
        Some(FileSidebar::new(self, meta).into_any_element())
    }

    #[inline(always)]
    fn has_sidebar(&self) -> bool {
        true
    }

    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        if self.loc.ends_with('/') {
            return Some(ExecMode::CreateView {
                mode: NavigationViewType::Files {
                    dir: Some(self.loc.clone()),
                },
                launcher: Arc::clone(launcher),
            });
        }
        None
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        None
    }

    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }

    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.loc
    }
}
