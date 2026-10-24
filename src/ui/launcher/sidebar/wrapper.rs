use std::sync::Arc;

use gpui::{
    AnyElement, Div, IntoElement, ParentElement, SharedString, StyleRefinement, Styled, div, img,
    px,
};

use crate::{app::theme::ThemeData, loader::IconType};

pub struct Sidebar {
    inner: Div,
    theme: Arc<ThemeData>,
}

impl Sidebar {
    pub fn new(theme: Arc<ThemeData>) -> Self {
        Self {
            inner: div().flex_col(),
            theme,
        }
    }

    pub fn separator(mut self) -> Self {
        self.inner = self.inner.child(
            div()
                .h(px(1.))
                .w_full()
                .bg(self.theme.border_selected)
                .my_1(),
        );
        self
    }

    pub fn section(mut self, label: &'static str) -> Self {
        self.inner = self.inner.child(
            div()
                .text_xs()
                .font_family(self.theme.font_family.clone())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(self.theme.secondary_text)
                .pt_3()
                .pb_1()
                .child(SharedString::from(label)),
        );
        self
    }

    pub fn row(mut self, label: &'static str, value: impl Into<SharedString>) -> Self {
        let t = &self.theme;
        let row = div()
            .flex()
            .items_start()
            .justify_between()
            .gap_4()
            .py(px(4.))
            .child(
                div()
                    .text_xs()
                    .text_color(t.secondary_text)
                    .flex_shrink_0()
                    .child(label),
            )
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(t.primary_text)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(value.into()),
            );
        self.inner = self.inner.child(row);
        self
    }

    fn badge(&self, text: SharedString) -> Div {
        let t = &self.theme;
        div()
            .px(px(6.))
            .py(px(1.))
            .rounded(px(4.))
            .bg(t.bg_muted)
            .text_xs()
            .font_family(t.font_family.clone())
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(t.secondary_text)
            .child(text)
    }

    pub fn header(
        mut self,
        icon: Option<&IconType>,
        title: impl Into<SharedString>,
        badges: impl IntoIterator<Item = SharedString>,
    ) -> Self {
        let t = self.theme.clone();
        let icon_el = match icon {
            Some(icon) => match icon.svg() {
                Some(svg) => svg
                    .size(px(32.))
                    .text_color(t.tertiary_text)
                    .into_any_element(),
                None => img(icon.clone()).size(px(32.)).into_any_element(),
            },
            None => div().into_any_element(),
        };
        let badges: Vec<_> = badges.into_iter().map(|b| self.badge(b)).collect();

        self.inner = self.inner.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .pb(px(12.))
                .child(
                    div()
                        .size(px(48.))
                        .rounded_lg()
                        .bg(t.bg_muted)
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_shrink_0()
                        .child(icon_el),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .h(px(48.))
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_family(t.font_family.clone())
                                .px(px(3.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(t.primary_text)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(title.into()),
                        )
                        .child(div().flex().items_center().gap_1().children(badges)),
                ),
        );
        self
    }

    pub fn location(mut self, path: impl Into<SharedString>) -> Self {
        self.inner = self.inner.child(
            div()
                .pt(px(5.))
                .text_xs()
                .text_color(self.theme.secondary_text)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(path.into()),
        );
        self
    }

    pub fn line(mut self, text: impl Into<SharedString>) -> Self {
        self.inner = self.inner.child(
            div()
                .py(px(4.))
                .text_xs()
                .font_family(self.theme.font_family.clone())
                .text_color(self.theme.primary_text)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(text.into()),
        );
        self
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl Styled for Sidebar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl IntoElement for Sidebar {
    type Element = Div;
    fn into_element(self) -> Self::Element {
        self.inner
    }
}
