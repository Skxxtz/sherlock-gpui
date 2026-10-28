use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor,
    App,
    AppContext,
    Bounds,
    Context,
    Entity,
    EventEmitter,
    FocusHandle,
    Focusable,
    Hsla,
    InteractiveElement,
    IntoElement,
    KeyDownEvent,
    Keystroke,
    MouseDownEvent,
    ParentElement,
    Pixels,
    Render,
    Stateful,
    StatefulInteractiveElement,
    Styled,
    Subscription,
    Window,
    anchored,
    deferred,
    div,
    point,
    prelude::FluentBuilder,
    px,
};

use super::{
    ColorEvent,
    PAD,
    PANEL_W,
    data::Hsv,
    render::{field, hue_slider, key_hint},
    state::ColorPickerState,
};
use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::{
        actions::{Execute, ExecuteInplace, ItemDown, ItemLeft, ItemRight, ItemUp, Quit},
        components::{
            color_swatch::{hex, parse_hex},
            tiny_input::{TextEvent, TinyInput},
        },
        traits::{HandleActions, HandleInputExt},
        views::Propagation,
    },
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

    hex: Entity<TinyInput>,
    editing: Option<Hsv>,
    hex_sub: Option<Subscription>,
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
        let hex = cx.new(|cx| {
            TinyInput::new(cx)
                .max_len(6)
                .filter(|_, _, c| c.is_ascii_hexdigit())
                .with_text(self::hex(color))
                .prefix("#")
        });
        Self {
            state: ColorPickerState::new(color),
            open: false,
            focus: cx.focus_handle(),
            anchor,

            hex,
            editing: None,
            hex_sub: None,
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
        self.sync_hex(cx);
        cx.notify();
    }

    /// The single way a pick changes (drag, keys, revert, hex, etc.)
    pub fn set_hsv(&mut self, hsv: Hsv, cx: &mut Context<Self>) {
        if hsv != self.state.hsv {
            self.state.hsv = hsv;
            self.sync_hex(cx);
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
        self.hex_sub = Some(cx.subscribe_in(&self.hex, win, Self::on_hex));
        cx.notify();
    }

    /// Closes the picker, keeping the color.
    pub fn close(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.hex_sub = None;
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

    /// Whether `keystroke` starts editing the hex: plain typing of `#` (the field's prefix, so
    /// it only starts the edit) or a char the field takes as its first one.
    fn starts_hex(&self, keystroke: &Keystroke, cx: &App) -> bool {
        if keystroke.modifiers.modified() {
            return false;
        }
        let Some(typed) = keystroke.key_char.as_deref() else {
            return false;
        };
        let hex = self.hex.read(cx);
        typed.chars().all(|c| c == '#' || hex.filter_accepts(c))
    }

    fn nudge(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut ColorPickerState),
    ) -> Propagation {
        let mut next = self.state.clone();
        f(&mut next);
        self.set_hsv(next.hsv, cx);
        Propagation::Stop
    }
}

impl HandleActions for ColorInput {
    /// A popup: nothing it doesn't use reaches the launcher.
    const UNHANDLED: Propagation = Propagation::Stop;

    fn item_left(&mut self, _: &ItemLeft, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.nudge(cx, |s| s.nudge_hue(-HUE_STEP))
    }
    fn item_right(&mut self, _: &ItemRight, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.nudge(cx, |s| s.nudge_hue(HUE_STEP))
    }
    fn item_up(&mut self, _: &ItemUp, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.nudge(cx, |s| s.nudge_value(VALUE_STEP))
    }
    fn item_down(&mut self, _: &ItemDown, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.nudge(cx, |s| s.nudge_value(-VALUE_STEP))
    }

    fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.close(win, cx);
        Propagation::Stop
    }
    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.close(win, cx);
        Propagation::Stop
    }
    fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.revert();
        self.close(win, cx);
        Propagation::Stop
    }
    /// Typing a hex digit (or `#`) starts an edit with that key. While editing, the field has
    /// focus and gets keys first; only what it passes on arrives here.
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.editing.is_none() && self.starts_hex(&event.keystroke, cx) {
            self.start_editing(win, cx);
            self.hex.update(cx, |i, cx| {
                i.set_text("", cx);
                i.key_down(event, win, cx)
            });
        }
        Propagation::Stop
    }
}

// TinyInput stuff
impl ColorInput {
    /// Syncs the hex input field to the current color
    fn sync_hex(&self, cx: &mut Context<Self>) {
        if self.editing.is_none() {
            let text = hex(self.state.color());
            self.hex.update(cx, |i, cx| i.set_text(&text[1..], cx));
        }
    }

    /// Starts editing the hex field.
    fn start_editing(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_none() {
            self.editing = Some(self.state.hsv);
            self.hex.focus_handle(cx).focus(win, cx);
            cx.notify();
        }
    }

    fn stop_editing(&mut self, restore: bool, win: &mut Window, cx: &mut Context<Self>) {
        let Some(before) = self.editing.take() else {
            return;
        };
        if restore {
            self.set_hsv(before, cx);
        }
        self.sync_hex(cx);

        if self.is_open() {
            self.focus.focus(win, cx);
        }
        cx.notify();
    }

    fn on_hex(
        &mut self,
        hex: &Entity<TinyInput>,
        event: &TextEvent,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // a valid code applies live; the field keeps what was typed
            TextEvent::Changed => {
                if let Some(color) = parse_hex(hex.read(cx).text()) {
                    self.set_hsv(Hsv::from_hsla(color), cx);
                }
            }
            TextEvent::Submit => self.stop_editing(false, win, cx),
            TextEvent::Cancel => self.stop_editing(true, win, cx),
        }
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
                            .id("hex")
                            .flex_1()
                            .h(px(28.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if self.editing.is_some() {
                                theme.accent()
                            } else {
                                theme.border
                            })
                            .cursor_text()
                            .font_family(theme.monospace.clone())
                            .text_size(px(13.))
                            .child(self.hex.clone())
                            .on_click(cx.listener(|this, _, win, cx| this.start_editing(win, cx))),
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
                    .map(|this| {
                        if self.editing.is_some() {
                            this.child(key_hint("esc", "restore", theme))
                                .child(div().flex_1())
                                .child(key_hint("↵", "apply", theme))
                        } else {
                            this.child(key_hint("←→", "hue", theme))
                                .child(key_hint("↑↓", "brightness", theme))
                                .child(div().flex_1())
                                .child(key_hint("↵", "done", theme))
                        }
                    }),
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
            .handle_input(cx)
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
