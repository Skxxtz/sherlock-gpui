use gpui::{
    Anchor, Context, Div, InteractiveElement, IntoElement, MouseDownEvent, ParentElement, Render,
    Stateful, StatefulInteractiveElement, Styled, Window, anchored, deferred, div, point,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::{components::keybind::bounds_probe, traits::HandleInputExt},
};

use super::{PAD, ROW_H, Select, VISIBLE};

impl Select {
    fn render_list(&self, theme: &ThemeData, cx: &mut Context<Self>) -> Stateful<Div> {
        let rows: Vec<_> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let (highlighted, chosen) = (i == self.cursor, i == self.selected);
                div()
                    .id(("option", i))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .h(px(ROW_H))
                    .px(px(10.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(highlighted, |this| this.bg(theme.bg_selected))
                    .child(
                        div()
                            .w(px(12.))
                            .text_color(theme.accent())
                            .when(chosen, |this| this.child("✓")),
                    )
                    .child(
                        div()
                            .text_color(if chosen {
                                theme.primary_text
                            } else {
                                theme.secondary_text
                            })
                            .child(label.to_lowercase()),
                    )
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered && this.cursor != i {
                            this.cursor = i;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, win, cx| {
                        this.choose(i, win, cx);
                        this.close(win, cx)
                    }))
            })
            .collect();

        div()
            .id("select-list")
            .occlude()
            .min_w(px(160.))
            .p(px(PAD))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.bg_app)
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, win, cx| {
                if !this.anchor.get().is_some_and(|b| b.contains(&e.position)) {
                    this.close(win, cx);
                }
            }))
            .child(
                div()
                    .id("select-rows")
                    .flex()
                    .flex_col()
                    .max_h(px(VISIBLE as f32 * ROW_H))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(rows),
            )
    }
}
impl Render for Select {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let id = cx.entity_id().as_u64() as usize;
        let label = self
            .options
            .get(self.selected)
            .map(|o| o.to_lowercase())
            .unwrap_or_default();
        let list = self.open.then(|| self.render_list(&theme, cx));

        div()
            .relative()
            .flex_none()
            .track_focus(&self.focus)
            .handle_input(cx)
            .child(
                div()
                    .id(("select", id))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .border_1()
                    .bg(theme.bg_app)
                    .border_color(if self.open {
                        theme.accent()
                    } else {
                        theme.border
                    })
                    .cursor_pointer()
                    .when(self.open, |this| this.bg(theme.bg_selected))
                    .hover(|s| s.bg(theme.bg_selected))
                    .font_family(theme.monospace.clone())
                    .text_size(px(12.))
                    .text_color(theme.primary_text)
                    .child(label)
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme.secondary_text)
                            .child(if self.open { "▴" } else { "▾" }),
                    )
                    .on_click(cx.listener(|this, _, win, cx| this.toggle(win, cx)))
                    .child(bounds_probe(self.anchor.clone())),
            )
            .when_some(list, |this, list| {
                // zero-size point at the control's bottom-right; the list hangs from it and flips
                // above near the win's bottom
                this.child(
                    div().absolute().right_0().bottom_0().size_0().child(
                        deferred(
                            anchored()
                                .anchor(Anchor::TopRight)
                                .offset(point(px(0.), px(6.)))
                                .child(
                                    div()
                                        .font_family(theme.monospace.clone())
                                        .text_size(px(12.))
                                        .child(list),
                                ),
                        )
                        .with_priority(1),
                    ),
                )
            })
    }
}
