use std::time::Duration;

use gpui::{
    Animation,
    AnimationExt,
    ClickEvent,
    Context,
    Div,
    ElementId,
    InteractiveElement,
    IntoElement,
    MouseDownEvent,
    ParentElement,
    Render,
    Stateful,
    StatefulInteractiveElement,
    Styled,
    Window,
    div,
    ease_out_quint,
    prelude::FluentBuilder,
    px,
};

use crate::{
    app::theme::{ActiveTheme, ThemeData},
    loader::resolve_icon_path,
    ui::traits::HandleInputExt,
};

const H: f32 = 28.;
const ICON: f32 = 12.;
const FIELD_W: f32 = 28.;
const FIELD_W_FOCUSED: f32 = 44.;
const GROW: Duration = Duration::from_millis(140);

impl super::NumberInput {
    fn chevron(
        &self,
        id: &'static str,
        icon: &'static str,
        steps: i64,
        enabled: bool,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let color = if enabled {
            theme.secondary_text
        } else {
            theme.tertiary_text
        };
        div()
            .id(id)
            .w(px(H))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .children(
                resolve_icon_path(icon)
                    .and_then(|i| i.svg())
                    .map(|svg| svg.size(px(ICON)).text_color(color)),
            )
            .when(enabled, |this| {
                this.cursor_pointer()
                    .hover(|s| s.bg(theme.bg_selected))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.step_by(steps, window, cx);
                    }))
            })
    }
}

impl Render for super::NumberInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let focused = self.entry.is_some();

        let field = div()
            .id("number")
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.))
            .line_height(px(16.))
            .text_color(theme.primary_text)
            .cursor_text()
            .child(self.field.clone())
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.start_typing(false, window, cx);
            }));
        // grows once per focus-in; shrinks back without animation
        let field = if focused {
            field
                .with_animation(
                    ElementId::from(("number-grow", self.focus_count)),
                    Animation::new(GROW).with_easing(ease_out_quint()),
                    |el, t| el.w(px(FIELD_W + (FIELD_W_FOCUSED - FIELD_W) * t)),
                )
                .into_any_element()
        } else {
            field.w(px(FIELD_W)).into_any_element()
        };

        let dec = self.chevron(
            "dec",
            "sherlock-chevron-left",
            -1,
            self.value > self.min,
            &theme,
            cx,
        );
        let inc = self.chevron(
            "inc",
            "sherlock-chevron-right",
            1,
            self.value < self.max,
            &theme,
            cx,
        );

        div()
            .track_focus(&self.focus)
            .handle_input(cx)
            .flex()
            .flex_none()
            .items_center()
            .h(px(H))
            .rounded(px(7.))
            .border_1()
            .border_color(if focused {
                theme.accent()
            } else {
                theme.border
            })
            .when(focused, |this| this.bg(theme.bg_selected))
            .overflow_hidden()
            .font_family(theme.monospace.clone())
            .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, window, cx| {
                if this.entry.is_some() {
                    this.leave(window, cx);
                }
            }))
            .child(dec)
            .child(field)
            .child(inc)
    }
}
