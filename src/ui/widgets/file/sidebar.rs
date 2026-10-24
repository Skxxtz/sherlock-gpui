use std::sync::Arc;

use gpui::{
    Div, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, StyledImage, div,
    img, prelude::FluentBuilder, px,
};

use super::FileMeta;
use crate::{
    app::theme::{ActiveTheme, ThemeData},
    loader::IconType,
    ui::{launcher::sidebar::wrapper::Sidebar, widgets::file::FileData},
    utils::{
        preview::{Preview, PreviewKind, PreviewState, THUMB_PX},
        preview_lru::PreviewCache,
    },
};

#[derive(IntoElement)]
pub struct FileSidebar {
    meta: FileMeta,
    loc: SharedString,
    name: SharedString,
    icon: Option<IconType>,
}

impl FileSidebar {
    pub fn new(value: &FileData, meta: FileMeta) -> Self {
        Self {
            meta,
            icon: value.icon.clone(),
            name: value.name.clone(),
            loc: value.loc.clone(),
        }
    }
}

impl RenderOnce for FileSidebar {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();

        let kind = self.meta.kind;
        let state = PreviewState::request(self.loc.clone(), kind, cx);

        cx.default_global::<PreviewCache>().flush(window);

        let badges = std::iter::once(SharedString::from(kind.as_str()))
            .chain(self.meta.executable.then(|| "Executable".into()));

        Sidebar::new(theme.clone())
            .min_h_full()
            .header(self.icon.as_ref(), self.name.clone(), badges)
            .location(self.loc.clone())
            .separator()
            .section("Info")
            .row("Size", self.meta.size.clone())
            .row("Kind", kind.as_str())
            .when_some(self.meta.symlink_target, |s, target| {
                s.row("Points to", target)
            })
            .separator()
            .section("Dates")
            .row("Modified", self.meta.modified)
            .row("Created", self.meta.created)
            .separator()
            .section("Permissions")
            .row("Mode", self.meta.permissions)
            .when(!matches!(state, PreviewState::Failed), |s| {
                s.separator()
                    .section("Preview")
                    .child(render_preview(state, &theme))
            })
    }
}

fn render_preview(state: PreviewState, theme: &Arc<ThemeData>) -> Div {
    match state {
        PreviewState::Ready(Preview::Image(image)) => div()
            .w_full()
            .mt(px(3.))
            .mb(px(12.))
            .rounded_md()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .child(
                img(image)
                    .w_full()
                    .h(px(200.))
                    .object_fit(gpui::ObjectFit::Contain),
            ),

        PreviewState::Ready(Preview::Text(text)) => div()
            .w_full()
            .mt(px(3.))
            .mb(px(12.))
            .p_2()
            .max_h(px(200.))
            .overflow_hidden()
            .rounded_md()
            .bg(theme.bg_muted)
            .font_family("monospace")
            .text_size(px(11.))
            .text_color(theme.secondary_text)
            .children(text.lines().map(|line| {
                div()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(SharedString::from(
                        if line.is_empty() { " " } else { line }.to_owned(),
                    ))
            })),
        PreviewState::Loading(PreviewKind::Thumbnail(_)) => placeholder(px(THUMB_PX as f32), theme),
        PreviewState::Loading(_) => placeholder(px(80.), theme),
        PreviewState::Failed => div(),
    }
}

fn placeholder(h: Pixels, theme: &Arc<ThemeData>) -> Div {
    div().w_full().h(h).rounded_md().bg(theme.bg_idle)
}
