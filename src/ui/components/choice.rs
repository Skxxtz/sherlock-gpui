use gpui::{
    Context, Element, FocusHandle, InteractiveElement, IntoElement, ParentElement, Pixels, Render,
    Styled, Subscription, TextRun, div, prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ActiveTheme,
    ui::{
        components::choice::builder::ChoiceInputBuilder, runtime_arguments::choice::ChoiceOption,
        utils::pango::CachedPango,
    },
};

mod actions;
pub mod builder;

pub struct Choice {
    // Events
    pub scope: Option<&'static str>,
    pub focus_handle: FocusHandle,
    pub _sub: Option<Subscription>,

    // Data
    pub options: Vec<ChoiceOption>,
    pub placeholder: CachedPango,
    pub selected: Option<usize>,
    pub is_opened: bool,

    // cache
    max_width: Option<Pixels>,
}

impl Choice {
    pub fn builder() -> ChoiceInputBuilder {
        ChoiceInputBuilder::new()
    }
    pub fn render_inner(&mut self, max_width: Pixels, cx: &mut Context<Self>) -> impl Element {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let content = self.selected.and_then(|idx| self.options.get(idx));

        // px(2. * 12.) -> Left and right padding (.px_3() equates to 12px each side)
        // px(8.)       -> gap_2
        // px(3.)       -> chevron width
        // px(3.)       -> Left + right border strokes (1.5px * 2)
        let max_w = max_width + px(2. * 12.) + px(8.) + px(3.) + px(3.);

        div()
            .relative()
            .child(
                div()
                    .id("choice_input")
                    // events
                    .key_context(self.scope.unwrap_or("choice_input"))
                    .track_focus(&self.focus_handle)
                    .on_action(cx.listener(Self::on_selection_up))
                    .on_action(cx.listener(Self::on_selection_down))
                    .on_action(cx.listener(Self::on_backspace))
                    // layout
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    // text
                    .line_height(px(12.))
                    .text_size(px(12.))
                    .text_color(theme.tertiary_text)
                    .font_family(theme.font_family.clone())
                    // sizes
                    .py(px(3.))
                    .h(px(20. + 3. * 2.)) // 26px
                    .px_3()
                    .min_w(max_w) // 3px borders + 1px rounding buffer
                    // borders
                    .border(px(1.5))
                    .border_color(theme.border_idle)
                    .rounded_md()
                    // selected
                    .when(self.is_opened, |this| {
                        this.border_color(theme.border_selected)
                            .bg(theme.bg_muted)
                            .text_color(theme.secondary_text)
                    })
                    // children
                    .child(if let Some(content) = content {
                        content.label.clone().prepared(&theme)
                    } else {
                        self.placeholder.clone().prepared(&theme)
                    })
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .text_color(theme.tertiary_text)
                            .child(div().w(px(3.)).child(if self.selected.is_some() {
                                "▴"
                            } else {
                                "▾"
                            })),
                    ),
            )
            .when(self.is_opened, |this| {
                this.child(
                    div()
                        .absolute()
                        .mt(px(5.))
                        .w(max_w) // 3px borders + 1px rounding buffer
                        .p(px(2.))
                        // borders
                        .border(px(1.5))
                        .border_color(theme.border_idle)
                        .rounded_md()
                        // bg
                        .bg(theme.bg_overlay)
                        .children(self.options.iter().enumerate().map(|(idx, option)| {
                            div()
                                .py(px(6.))
                                .px_3()
                                .rounded_md()
                                .bg(theme.bg_idle)
                                .when(Some(idx) == self.selected, |this| {
                                    this.bg(theme.bg_selected)
                                })
                                .line_height(px(12.))
                                .text_size(px(12.))
                                .text_color(theme.secondary_text)
                                .font_family(theme.font_family.clone())
                                .child(option.label.clone().prepared(&theme))
                        })),
                )
            })
    }
}

impl Render for Choice {
    fn render(&mut self, win: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let style = win.text_style();
        let font_size = px(12.);
        let run = TextRun {
            len: 0,
            font: style.font(),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        if self.max_width.is_none() {
            let theme = cx.global::<ActiveTheme>().0.clone();
            for option in self.options.iter_mut() {
                option.label.populate(&theme);
            }
            self.placeholder.populate(&theme);

            self.max_width = self
                .options
                .iter_mut()
                .map(|o| o.label.text.as_ref())
                .chain(std::iter::once(self.placeholder.text.as_ref()))
                .map(|s| {
                    win.text_system()
                        .shape_line(
                            s.into(),
                            font_size,
                            &[TextRun {
                                len: s.len(),
                                ..run.clone()
                            }],
                            None,
                        )
                        .width
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        }

        self.is_opened = self.focus_handle.contains_focused(win, cx);
        self.render_inner(self.max_width.unwrap_or(px(60.)), cx)
    }
}
