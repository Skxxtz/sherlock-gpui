use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor, AsKeystroke, Bounds, Context, Div, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseDownEvent, ParentElement, Pixels, Render, ScrollHandle,
    SharedString, Stateful, StatefulInteractiveElement, Styled, Subscription, Window, anchored,
    deferred, div, point, prelude::FluentBuilder, px,
};

use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::{
        actions::{
            launcher::{Execute, ExecuteInplace, ItemDown, ItemLeft, ItemRight, ItemUp, Quit},
            search_bar::Backspace,
        },
        components::{
            keybind::{LINE_GAP, LINE_H, VISIBLE_LINES, caps, chord_label},
            keymap_input::state::KeymapState,
        },
    },
};

use super::KeymapEvent;

pub struct KeymapInput {
    title: SharedString,
    state: KeymapState,
    /// Shown per chord in the editor. e.g. "also Previous item"; set by the owner.
    notes: Vec<Option<String>>,
    open: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// Where the summary is
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Catches every keystroke before the keybinds while recording.
    intercept: Option<Subscription>,
}

impl EventEmitter<KeymapEvent> for KeymapInput {}

impl Focusable for KeymapInput {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl KeymapInput {
    /// Create a new KeymapInput
    #[inline(always)]
    pub fn new(
        title: impl Into<SharedString>,
        chords: Vec<SharedString>,
        anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            state: KeymapState::new(chords),
            notes: Vec::new(),
            open: false,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            anchor,
            intercept: None,
        }
    }

    /// Returns a slice containing all chords.
    #[inline(always)]
    pub fn chords(&self) -> &[SharedString] {
        &self.state.chords
    }

    /// Replaces the chords without emitting `KeymapEvent::Changed`.
    #[allow(dead_code)]
    #[inline(always)]
    pub fn set_chords(&mut self, chords: Vec<SharedString>, cx: &mut Context<Self>) {
        self.state = KeymapState::new(chords);
        cx.notify();
    }

    /// One note per chord (`None`: nothing to say). A note also marks the summary.
    #[inline(always)]
    pub fn set_notes(&mut self, notes: Vec<Option<String>>, cx: &mut Context<Self>) {
        if notes != self.notes {
            self.notes = notes;
            cx.notify();
        }
    }

    /// The notes for the configured chords.
    #[inline(always)]
    pub fn notes(&self) -> &[Option<String>] {
        &self.notes
    }

    /// Whether the input shows.
    #[inline(always)]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the editor and takes focus, so that keyboard drives it.
    pub fn open(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.state.set_cursor(0);
        self.focus.focus(win, cx);
        cx.notify();
    }

    /// Closes the editor and gives focus back (default focus target.)
    pub fn close(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.stop_recording();
        if self.focus.is_focused(win) {
            win.blur();
        }
        cx.notify();
    }

    /// Toggles open state for the window
    #[inline]
    pub fn toggle(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(win, cx);
        } else {
            self.open(win, cx)
        }
    }

    /// Sets the cursor position and scrolls to the item
    #[inline]
    fn set_cursor(&mut self, line: usize) {
        self.state.set_cursor(line);
        if self.state.cursor < self.state.chords.len() {
            self.scroll.scroll_to_item(self.state.cursor);
        }
    }

    /// Stops recording a keystroke
    #[inline(always)]
    fn stop_recording(&mut self) {
        self.state.recording = None;
        self.intercept = None;
    }

    /// Starts a recording for a chord
    fn start_recording(&mut self, line: usize, cx: &mut Context<Self>) {
        // Clicking recording line again cancels
        if self.state.recording == Some(line) {
            self.stop_recording();
            return;
        }

        self.state.record_at(line);
        let this = cx.entity().downgrade();
        self.intercept = Some(cx.intercept_keystrokes(move |event, _, cx| {
            cx.stop_propagation();
            let keystroke = &event.keystroke;
            let _ = this.update(cx, |this, cx| {
                let changed = this.state.record(keystroke);
                if this.state.recording.is_none() {
                    this.intercept = None;
                }
                this.set_cursor(this.state.cursor);
                if changed {
                    cx.emit(KeymapEvent::Changed);
                }
                cx.notify();
            });
        }));
    }

    /// Deletes a keybind
    fn remove(&mut self, i: usize, cx: &mut Context<Self>) {
        self.intercept = None;
        if self.state.remove(i) {
            cx.emit(KeymapEvent::Changed);
        }
        self.set_cursor(self.state.cursor);
    }
}

// Handles launcher actions while the editor has focus
impl KeymapInput {
    #[inline(always)]
    fn up(&mut self, _: &ItemUp, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(self.state.cursor.saturating_sub(1));
        cx.notify();
    }
    #[inline(always)]
    fn down(&mut self, _: &ItemDown, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(self.state.cursor.saturating_add(1));
        cx.notify();
    }
    #[inline(always)]
    fn confirm(&mut self, _: &Execute, _: &mut Window, cx: &mut Context<Self>) {
        self.start_recording(self.state.cursor, cx);
        cx.notify();
    }
    #[inline(always)]
    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.remove(self.state.cursor, cx);
        cx.notify();
    }
    #[inline(always)]
    fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) {
        self.close(win, cx);
    }
}

impl Render for KeymapInput {
    fn render(&mut self, win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .right_0()
            .bottom_0()
            .size_0()
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::quit))
            .on_action(
                cx.listener(|this, _: &ExecuteInplace, win, cx| this.confirm(&Execute, win, cx)),
            )
            .on_action(cx.listener(|this, _: &ItemLeft, win, cx| this.up(&ItemUp, win, cx)))
            .on_action(cx.listener(|this, _: &ItemRight, win, cx| this.down(&ItemDown, win, cx)))
            .when(self.open, |this| {
                let theme = cx.global::<ActiveTheme>().0.clone();
                this.child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .offset(point(px(0.), px(6.)))
                            .child(self.render_panel(&theme, win, cx)),
                    )
                    .with_priority(1),
                )
            })
    }
}

// The floating editor
impl KeymapInput {
    /// One editor line: the base for the chord lines and the "Add binding" line.
    fn line(&self, i: usize, theme: &ThemeData) -> Stateful<Div> {
        let selected = i == self.state.cursor;
        div()
            .id(("line", i))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .h(px(LINE_H))
            .px(px(10.))
            .rounded(px(7.))
            .cursor_pointer()
            .when(selected, |this| this.bg(theme.bg_selected))
            .when(!selected, |this| this.hover(|s| s.bg(theme.bg_selected)))
    }

    fn recording_label(theme: &ThemeData) -> Div {
        div()
            .flex_1()
            .text_size(px(12.))
            .text_color(theme.accent())
            .child("Press a key combination…")
    }

    fn small_button(
        id: (&'static str, usize),
        label: &'static str,
        theme: &ThemeData,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .px(px(7.))
            .h(px(22.))
            .flex()
            .items_center()
            .rounded(px(5.))
            .text_size(px(11.))
            .text_color(theme.secondary_text)
            .hover(|s| s.bg(theme.bg_keybind).text_color(theme.primary_text))
            .child(label)
    }

    /// A chord line: caps (or the recording prompt), its note, "Change" and "×".
    fn render_chord(
        &self,
        i: usize,
        chord: &str,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let content = if self.state.recording == Some(i) {
            Self::recording_label(theme).into_any_element()
        } else {
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(10.))
                .child(caps(chord, theme))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(px(11.))
                        .text_color(theme.color_warn)
                        .children(self.notes.get(i).cloned().flatten()),
                )
                .into_any_element()
        };

        self.line(i, theme)
            .child(content)
            .child(
                Self::small_button(("change", i), "Change", theme).on_click(cx.listener(
                    move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.start_recording(i, cx);
                        cx.notify();
                    },
                )),
            )
            .child(
                Self::small_button(("remove", i), "×", theme).on_click(cx.listener(
                    move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.remove(i, cx);
                        cx.notify();
                    },
                )),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.start_recording(i, cx);
                cx.notify();
            }))
    }

    fn render_add(&self, theme: &ThemeData, cx: &mut Context<Self>) -> Stateful<Div> {
        let at = self.state.add_line();
        self.line(at, theme)
            .map(|this| {
                if self.state.recording == Some(at) {
                    this.child(Self::recording_label(theme))
                } else {
                    this.child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.secondary_text)
                            .child("+  Add binding"),
                    )
                }
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.start_recording(this.state.add_line(), cx);
                cx.notify();
            }))
    }

    fn render_hints(&self, theme: &ThemeData, win: &Window) -> Div {
        let key = |action: &dyn gpui::Action| -> Option<String> {
            let binding = win.highest_precedence_binding_for_action(action)?;
            match binding.keystrokes() {
                [k] => Some(chord_label(&k.as_keystroke().unparse())),
                _ => None,
            }
        };
        let hint =
            |action: &dyn gpui::Action, label: &str| key(action).map(|k| format!("{k} {label}"));

        div()
            .flex()
            .gap(px(12.))
            .mt(px(4.))
            .px(px(10.))
            .pt(px(8.))
            .pb(px(4.))
            .border_t_1()
            .border_color(theme.border)
            .text_size(px(11.))
            .text_color(theme.tertiary_text)
            .justify_between()
            .map(|this| {
                if self.state.recording.is_some() {
                    // the recorder reads raw keys here, not actions
                    this.child("esc cancel").child("⌫ remove")
                } else {
                    this.children(hint(&Execute, "change"))
                        .children(hint(&Backspace, "remove"))
                        .children(hint(&Quit, "close"))
                }
            })
    }

    /// The panel: header, chord lines (scroll after `VISIBLE_LINES`), add line, hints.
    fn render_panel(
        &self,
        theme: &ThemeData,
        win: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let chords: Vec<_> = self
            .state
            .chords
            .iter()
            .enumerate()
            .map(|(i, chord)| self.render_chord(i, chord, theme, cx))
            .collect();

        div()
            .id("keymap-editor")
            .occlude()
            .w(px(300.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(px(6.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.bg_app)
            .shadow_lg()
            .font_family(theme.font_family.clone())
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, win, cx| {
                // clicks on the summary toggle it themselves
                if !this.anchor.get().is_some_and(|b| b.contains(&e.position)) {
                    this.close(win, cx);
                }
            }))
            .child(
                div()
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .text_color(theme.tertiary_text)
                    .child(format!("Bindings for {}", self.title)),
            )
            .child(
                div()
                    .id("lines")
                    .flex()
                    .flex_col()
                    .gap(px(LINE_GAP))
                    .max_h(px(
                        VISIBLE_LINES as f32 * LINE_H + (VISIBLE_LINES - 1) as f32 * LINE_GAP
                    ))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(chords),
            )
            .child(self.render_add(theme, cx))
            .child(self.render_hints(theme, win))
    }
}
