use std::sync::Arc;

use super::FileMeta;
use crate::{
    app::theme::{ActiveTheme, ThemeData},
    loader::IconType,
    ui::{launcher::sidebar::wrapper::Sidebar, widgets::file::FileData},
    utils::{filekinds::FileCategory, preview_lru::PreviewCache},
};
use gpui::{
    Div, IntoElement, ParentElement, RenderImage, RenderOnce, SharedString, Styled, StyledImage,
    div, img, prelude::FluentBuilder, px,
};

enum Preview {
    Image(Arc<RenderImage>),
    Text(SharedString),
}

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
        let preview = if kind.has_image_preview() {
            crate::utils::preview::request_preview(self.loc.clone(), cx).map(Preview::Image)
        } else if kind.is_text() || kind.category() == FileCategory::Other {
            crate::utils::preview::read_text_preview(self.loc.as_str(), 4096, 20)
                .map(SharedString::from)
                .map(Preview::Text)
        } else {
            None
        };
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
            .when_some(preview, |s, preview| {
                s.separator()
                    .section("Preview")
                    .child(render_preview(preview, &theme))
            })
    }
}

fn render_preview(preview: Preview, theme: &ThemeData) -> Div {
    match preview {
        Preview::Image(image) => div()
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

        Preview::Text(text) => div()
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
    }
}
