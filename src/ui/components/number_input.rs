use gpui::{
    AppContext,
    Context,
    Entity,
    EventEmitter,
    FocusHandle,
    Focusable,
    Subscription,
    Window,
};

use crate::ui::components::tiny_input::{TextEvent, TinyInput};

mod actions;
mod render;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberEvent {
    /// The value changed.
    Changed(i64),
    /// Editing ended (↵, Esc, click outside, etc.)
    Done,
}

pub struct NumberInput {
    value: i64,
    min: i64,
    max: i64,
    step: i64,

    focus: FocusHandle,
    /// Shows the value
    field: Entity<TinyInput>,
    /// While focused: the value focus came in with
    entry: Option<i64>,
    /// While typing: the value before typing
    typing: Option<i64>,
    /// The field's while focused
    field_sub: Option<Subscription>,
    /// Counts focus-ins; keys the grow animation so it plays once per focus
    focus_count: usize,
}

impl EventEmitter<NumberEvent> for NumberInput {}

impl Focusable for NumberInput {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Digits on 'n'
fn digits(n: i64) -> usize {
    n.unsigned_abs().to_string().len()
}

impl NumberInput {
    pub fn new(value: i64, min: i64, max: i64, step: i64, cx: &mut Context<Self>) -> Self {
        let (min, max) = (min.min(max), min.max(max));
        let value = value.clamp(min, max);
        let max_len = digits(min).max(digits(max)) + usize::from(min < 0);
        let field = cx.new(|cx| {
            TinyInput::new(cx)
                .max_len(max_len)
                .filter(|text, at, c| {
                    c.is_ascii_digit() || (c == '-' && at == 0 && !text.starts_with('-'))
                })
                .with_text(value.to_string())
        });
        Self {
            value,
            min,
            max,
            step: step.max(1),
            focus: cx.focus_handle(),
            field,
            entry: None,
            typing: None,
            field_sub: None,
            focus_count: 0,
        }
    }

    #[allow(dead_code)] // component API
    pub fn value(&self) -> i64 {
        self.value
    }

    /// Replaces the value (clamped) without emitting.
    pub fn set_value(&mut self, value: i64, cx: &mut Context<Self>) {
        self.value = value.clamp(self.min, self.max);
        self.typing = None;
        self.sync_field(cx);
        cx.notify();
    }

    #[allow(dead_code)] // component API
    pub fn is_focused(&self) -> bool {
        self.entry.is_some()
    }

    /// Takes focus: the keys go here until ↵, Esc or a click outside.
    pub fn focus(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.entry.is_none() {
            self.entry = Some(self.value);
            self.focus_count += 1;
            self.field_sub = Some(cx.subscribe_in(&self.field, win, Self::on_field));
        }
        self.focus.focus(win, cx);
        cx.notify();
    }

    /// Leaves: applies what was typed, gives focus back to the launcher.
    pub fn leave(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.typing.is_some() {
            self.apply_typed(cx);
        }
        if self.entry.take().is_some() {
            cx.emit(NumberEvent::Done);
        }
        self.field_sub = None;
        if self.focus.contains_focused(win, cx) {
            win.blur();
        }
        cx.notify();
    }

    /// The one way the value changes: clamps, emits if it changed, shows it.
    fn set(&mut self, value: i64, cx: &mut Context<Self>) {
        let value = value.clamp(self.min, self.max);
        if value != self.value {
            self.value = value;
            cx.emit(NumberEvent::Changed(value));
            if self.entry.is_none() {
                cx.emit(NumberEvent::Done);
            }
        }
        self.sync_field(cx);
        cx.notify();
    }

    /// Steps from the value. While typing, the typed text is dropped and the keys come back
    /// here.
    fn step_by(&mut self, steps: i64, win: &mut Window, cx: &mut Context<Self>) {
        if self.typing.take().is_some() && self.entry.is_some() {
            self.focus.focus(win, cx);
        }
        self.set(
            self.value.saturating_add(steps.saturating_mul(self.step)),
            cx,
        );
    }

    /// The field shows the value, except while the user is typing.
    fn sync_field(&self, cx: &mut Context<Self>) {
        if self.typing.is_none() {
            let text = self.value.to_string();
            self.field.update(cx, |field, cx| field.set_text(text, cx));
        }
    }

    /// Typing starts: the field gets focus (for the caret) and every key.
    fn start_typing(&mut self, clear: bool, win: &mut Window, cx: &mut Context<Self>) {
        self.focus(win, cx);
        if self.typing.is_none() {
            self.typing = Some(self.value);
            if clear {
                self.field.update(cx, |field, cx| field.set_text("", cx));
            }
        }
        self.field.focus_handle(cx).focus(win, cx);
        cx.notify();
    }

    /// Applies the typed number (clamped); an empty or lone `-` field keeps the value.
    fn apply_typed(&mut self, cx: &mut Context<Self>) {
        let typed = self.field.read(cx).text().parse::<i64>().ok();
        self.typing = None;
        match typed {
            Some(value) => self.set(value, cx),
            None => self.sync_field(cx),
        }
    }

    /// Handle field events
    fn on_field(
        &mut self,
        _: &Entity<TinyInput>,
        event: &TextEvent,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextEvent::Changed => return,
            TextEvent::Submit => self.apply_typed(cx),
            TextEvent::Cancel => {
                if let Some(before) = self.typing.take() {
                    self.set(before, cx);
                }
            }
        }
        if self.entry.is_some() {
            self.focus.focus(win, cx);
        }
    }
}
