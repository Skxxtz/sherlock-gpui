use std::{any::Any, cell::Cell, rc::Rc};

use gpui::{
    Anchor, App, Bounds, Context, EventEmitter, FocusHandle, Focusable, Hsla, InteractiveElement,
    IntoElement, Keystroke, MouseDownEvent, ParentElement, Pixels, Render, Stateful,
    StatefulInteractiveElement, Styled, Subscription, Window, anchored, deferred, div, point,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::{
        components::{color_swatch::hex, keyboard_grab::grab_keyboard},
        launcher::{
            Execute, Quit, SelectionDown, SelectionLeft, SelectionRight, SelectionUp,
            actions::ExecuteInplace,
        },
    },
};

use super::{
    ColorEvent, PAD, PANEL_W,
    data::Hsv,
    render::{field, hue_slider, key_hint},
    state::ColorPickerState,
};

const HUE_STEP: f32 = 1. / 72.;
const VALUE_STEP: f32 = 0.05;

pub struct ColorInput {
    state: ColorPickerState,
    /// Open state
    open: bool,
    focus: FocusHandle,
    /// The element picker bounds
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Holds the keyboard while open.
    grab: Option<Subscription>,
}

impl EventEmitter<ColorEvent> for ColorInput {}

impl Focusable for ColorInput {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl ColorInput {
    /// Creates a new ColorInput instance.
    pub fn new(
        color: Hsla,
        anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            state: ColorPickerState::new(color),
            open: false,
            focus: cx.focus_handle(),
            anchor,
            grab: None,
        }
    }

    /// Gets the currently selected color.
    #[inline(always)]
    pub fn color(&self) -> Hsla {
        self.state.color()
    }

    /// Replaces the color without emitting.
    #[inline(always)]
    pub fn set_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.state = ColorPickerState::new(color);
        cx.notify();
    }

    /// The single way a pick changes (drag, keys, revert, hex, etc.)
    pub fn set_hsv(&mut self, hsv: Hsv, cx: &mut Context<Self>) {
        if hsv != self.state.hsv {
            self.state.hsv = hsv;
            cx.emit(ColorEvent::Changed(self.state.color()));
            cx.notify();
        }
    }

    /// Whether the picker input is opened.
    #[inline(always)]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the picker on the current color
    pub fn open(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.state = ColorPickerState::new(self.state.color());
        self.focus.focus(win, cx);
        self.grab = Some(grab_keyboard(cx, Self::on_key));
        cx.notify();
    }

    /// Closes the picker, keeping the color.
    pub fn close(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.grab = None;
        if self.focus.is_focused(win) {
            win.blur();
        }
        cx.notify();
    }

    /// Toggles the picker open state
    #[inline(always)]
    pub fn toggle(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.is_open() {
            self.close(win, cx);
        } else {
            self.open(win, cx)
        }
    }

    /// Back to the color the picker was opened with.
    #[inline(always)]
    pub fn revert(&mut self) {
        self.state.revert();
    }

    /// Keyboard while open. Swallows every key
    fn on_key(
        &mut self,
        _: &Keystroke,
        action: Option<&dyn Any>,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(action) = action else {
            return true;
        };

        let mut next = self.state.clone();
        if action.is::<SelectionLeft>() {
            next.nudge_hue(-HUE_STEP);
        } else if action.is::<SelectionRight>() {
            next.nudge_hue(HUE_STEP);
        } else if action.is::<SelectionUp>() {
            next.nudge_value(VALUE_STEP);
        } else if action.is::<SelectionDown>() {
            next.nudge_value(-VALUE_STEP);
        } else if action.is::<Execute>() || action.is::<ExecuteInplace>() {
            self.close(win, cx);
            return true;
        } else if action.is::<Quit>() {
            self.revert();
            self.close(win, cx);
            return true;
        }

        self.set_hsv(next.hsv, cx);
        true
    }
}

impl ColorInput {
    fn render_panel(&self, theme: &ThemeData, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let color = self.state.color();
        let field_drag = cx.listener(|this, &(x, y): &(f32, f32), _, cx| {
            let hsv = Hsv {
                s: x,
                v: 1. - y,
                ..this.state.hsv
            };
            this.set_hsv(hsv, cx);
        });
        let hue_drag = cx.listener(|this, &(x, _): &(f32, f32), _, cx| {
            let hsv = Hsv {
                h: x.min(0.999),
                ..this.state.hsv
            };
            this.set_hsv(hsv, cx);
        });
        let preview = |color: Hsla| div().flex_1().h_full().bg(color);

        div()
            .id("color-picker")
            .occlude()
            .w(px(PANEL_W))
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(PAD))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.bg_app)
            .shadow_lg()
            .font_family(theme.font_family.clone())
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, window, cx| {
                // the owner's swatch toggles the picker itself
                if !this.anchor.get().is_some_and(|b| b.contains(&e.position)) {
                    this.close(window, cx);
                }
            }))
            // before | after, hex (the future hex field goes here)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .id("before")
                            .flex()
                            .w(px(56.))
                            .h(px(28.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(preview(self.state.original).rounded_l(px(5.)))
                            .child(preview(color).rounded_r(px(5.)))
                            .on_click(cx.listener(|this, _, _, _| this.revert())),
                    )
                    .child(
                        div()
                            .font_family(theme.monospace.clone())
                            .text_size(px(13.))
                            .text_color(theme.primary_text)
                            .child(hex(color)),
                    ),
            )
            .child(field(
                &self.state,
                theme,
                Rc::new(move |x, y, window: &mut Window, cx: &mut App| {
                    field_drag(&(x, y), window, cx)
                }),
            ))
            .child(hue_slider(
                &self.state,
                theme,
                Rc::new(move |x, y, window: &mut Window, cx: &mut App| {
                    hue_drag(&(x, y), window, cx)
                }),
            ))
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(theme.border)
                    .text_size(px(11.))
                    .text_color(theme.secondary_text)
                    .child(key_hint("←→", "hue", theme))
                    .child(key_hint("↑↓", "brightness", theme))
                    .child(div().flex_1())
                    .child(key_hint("↵", "done", theme)),
            )
    }
}

impl Render for ColorInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let panel = self.open.then(|| self.render_panel(&theme, cx));

        // Always tracked, even closed: the launcher's focus check uses the last frame's tree.
        // Zero-size point at the owner's bottom-right; the panel hangs from it.
        div()
            .absolute()
            .right_0()
            .bottom_0()
            .size_0()
            .track_focus(&self.focus)
            // the grab only sees key downs
            .on_key_up(|_, _, cx| cx.stop_propagation())
            .when_some(panel, |this, panel| {
                this.child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .offset(point(px(0.), px(6.)))
                            .child(panel),
                    )
                    .with_priority(1),
                )
            })
    }
}
