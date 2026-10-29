use gpui::{
    AnyElement,
    App,
    AppContext,
    Context,
    Entity,
    FocusHandle,
    Focusable,
    InteractiveElement,
    IntoElement,
    ParentElement,
    SharedString,
    StatefulInteractiveElement,
    Styled,
    Subscription,
    Window,
    div,
    px,
};

use super::{
    SettingsView,
    model::{Edit, Setting, SettingValue},
};
use crate::{
    app::theme::ThemeData,
    ui::components::{
        color_picker::{ColorEvent, ColorItem},
        keymap_input::KeymapEvent,
        keymap_item::KeymapItem,
        number_input::{NumberEvent, NumberInput},
        select::{Select, SelectEvent},
        tiny_input::{TextEvent, TinyInput},
    },
};

/// The editor of one setting. Its changes come back to the view as [`Edit`]s for the setting's
/// id; the view's `sections` stay the source of truth.
pub(super) enum Control {
    Color(Entity<ColorItem>),
    Select(Entity<Select>),
    Keymap(Entity<KeymapItem>),
    Number(Entity<NumberInput>),
    /// A text field; the focus handle lets a click on its frame focus it.
    Text(Entity<TinyInput>, FocusHandle),
}

impl Control {
    /// The editor for `setting`, wired to `SettingsView::apply`. `None` for plain values.
    pub(super) fn new(
        setting: &Setting,
        cx: &mut Context<SettingsView>,
    ) -> Option<(Self, Subscription)> {
        let id = setting.id;
        Some(match &setting.value {
            SettingValue::Color(color) => {
                let color = *color;
                let item = cx.new(|cx| ColorItem::new(color, cx));
                let sub = cx.subscribe(&item, move |this, _, event: &ColorEvent, cx| {
                    let ColorEvent::Changed(color) = *event;
                    this.apply(id, Edit::SetColor(color), cx);
                });
                (Self::Color(item), sub)
            }
            SettingValue::Choice { options, selected } => {
                let options = options.iter().map(|&o| SharedString::from(o)).collect();
                let selected = *selected;
                let select = cx.new(|cx| Select::new(options, selected, cx));
                let sub = cx.subscribe(&select, move |this, _, event: &SelectEvent, cx| {
                    let SelectEvent::Changed(i) = *event;
                    this.apply(id, Edit::SetChoice(i), cx);
                });
                (Self::Select(select), sub)
            }
            SettingValue::Keybind(chords) => {
                let (label, chords) = (setting.label, chords.clone());
                let item = cx.new(|cx| KeymapItem::new(label, chords, cx));
                let sub = cx.subscribe(&item, move |this, item, _: &KeymapEvent, cx| {
                    let chords = item.read(cx).chords(cx).to_vec();
                    this.apply(id, Edit::SetChords(chords), cx);
                });
                (Self::Keymap(item), sub)
            }
            SettingValue::Number {
                value,
                min,
                max,
                step,
            } => {
                let (value, min, max, step) = (*value, *min, *max, *step);
                let input = cx.new(|cx| NumberInput::new(value, min, max, step, cx));
                // steps apply live; saved once editing ends
                let sub = cx.subscribe(
                    &input,
                    move |this, _, event: &NumberEvent, cx| match *event {
                        NumberEvent::Changed(value) => this.edit(id, Edit::SetNumber(value), cx),
                        NumberEvent::Done => this.save(cx, false),
                    },
                );
                (Self::Number(input), sub)
            }
            SettingValue::Text(text) => {
                let text = text.clone();
                let input = cx.new(|cx| TinyInput::new(cx).with_text(text));
                let focus = input.read(cx).focus_handle(cx);
                // ↵ applies and saves the typed text, Esc restores the value; both leave the field
                let sub = cx.subscribe(&input, move |this, input, event: &TextEvent, cx| {
                    match event {
                        TextEvent::Changed => return,
                        TextEvent::Submit => {
                            let text = SharedString::from(input.read(cx).text().to_string());
                            this.apply(id, Edit::SetText(text), cx);
                        }
                        TextEvent::Cancel => this.restore(id, cx),
                    }
                    // events are delivered after the keystroke, when the window is free
                    let _ = cx.with_window(cx.entity_id(), |window, _| window.blur());
                });
                (Self::Text(input, focus), sub)
            }
            SettingValue::Toggle(_) => return None,
        })
    }

    /// Enter on the row: opens the editor (it takes the keyboard while open).
    pub(super) fn open(&self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Color(item) => item.update(cx, |item, cx| item.open(window, cx)),
            Self::Select(select) => select.update(cx, |select, cx| select.open(window, cx)),
            Self::Keymap(item) => item.update(cx, |item, cx| item.open(window, cx)),
            Self::Number(input) => input.update(cx, |input, cx| input.focus(window, cx)),
            Self::Text(_, focus) => focus.focus(window, cx),
        }
    }

    /// Shows `value` again after a change from outside the editor (reset, reload). Doesn't emit,
    /// so it doesn't come back as an edit.
    pub(super) fn sync(&self, value: &SettingValue, cx: &mut App) {
        match (self, value) {
            (Self::Color(item), SettingValue::Color(color)) => {
                item.update(cx, |item, cx| item.set_color(*color, cx))
            }
            (Self::Select(select), SettingValue::Choice { selected, .. }) => {
                select.update(cx, |select, cx| select.set_selected(*selected, cx))
            }
            (Self::Keymap(item), SettingValue::Keybind(chords)) => {
                let input = item.read(cx).input().clone();
                input.update(cx, |input, cx| input.set_chords(chords.clone(), cx));
            }
            (Self::Number(input), SettingValue::Number { value, .. }) => {
                input.update(cx, |input, cx| input.set_value(*value, cx))
            }
            (Self::Text(input, _), SettingValue::Text(text)) => {
                input.update(cx, |input, cx| input.set_text(text, cx))
            }
            _ => {}
        }
    }

    /// Notes per chord for a key recorder ("also Previous item"); ignored by other editors.
    pub(super) fn set_notes(&self, notes: Vec<Option<String>>, cx: &mut App) {
        if let Self::Keymap(item) = self {
            item.update(cx, |item, cx| item.set_notes(notes, cx));
        }
    }

    pub(super) fn element(&self, theme: &ThemeData) -> AnyElement {
        match self {
            Self::Color(item) => item.clone().into_any_element(),
            Self::Select(select) => select.clone().into_any_element(),
            Self::Keymap(item) => item.clone().into_any_element(),
            Self::Number(input) => input.clone().into_any_element(),
            Self::Text(input, focus) => {
                let focus = focus.clone();
                div()
                    .id(("text-field", input.entity_id().as_u64() as usize))
                    .w(px(200.))
                    .h(px(28.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .rounded(px(7.))
                    .border_1()
                    .border_color(theme.border)
                    .cursor_text()
                    .font_family(theme.monospace.clone())
                    .text_size(px(12.))
                    .child(input.clone())
                    .on_click(move |_, window, cx| focus.focus(window, cx))
                    .into_any_element()
            }
        }
    }
}
