use std::{cell::Cell, rc::Rc};

use gpui::{
    App, AppContext, Bounds, Context, Entity, EventEmitter, IntoElement, ParentElement, Pixels,
    Render, SharedString, Styled, Subscription, Window, div,
};

use crate::{
    app::theme::ActiveTheme,
    ui::components::{
        keybind::{bounds_probe, keybind_summary},
        keymap_input::{KeymapEvent, KeymapInput},
    },
};

/// The component. See the module docs. Re-emits the input's [`KeymapEvent`]s.
pub struct KeymapItem {
    input: Entity<KeymapInput>,
    /// The summary's bounds, shared with the input.
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    _subs: [Subscription; 2],
}

impl EventEmitter<KeymapEvent> for KeymapItem {}

impl KeymapItem {
    pub fn new(
        title: impl Into<SharedString>,
        chords: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let anchor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        let title = title.into();
        let input = cx.new(|cx| KeymapInput::new(title, chords, anchor.clone(), cx));
        let _subs = [
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.subscribe(&input, |_, _, event: &KeymapEvent, cx| cx.emit(*event)),
        ];
        Self {
            input,
            anchor,
            _subs,
        }
    }

    /// The editor this item opens.
    #[allow(dead_code)] // component API
    pub fn input(&self) -> &Entity<KeymapInput> {
        &self.input
    }

    pub fn chords<'a>(&self, cx: &'a App) -> &'a [SharedString] {
        self.input.read(cx).chords()
    }

    /// One note per chord, shown in the editor; any note also marks the summary.
    pub fn set_notes(&mut self, notes: Vec<Option<String>>, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.set_notes(notes, cx));
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.open(window, cx));
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.toggle(window, cx));
    }
}

impl Render for KeymapItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let id = cx.entity_id().as_u64() as usize;
        let input = self.input.read(cx);
        let warn = input.notes().iter().any(Option::is_some);
        let open = input.is_open();

        let toggle = cx.listener(|this, _: &(), window, cx| this.toggle(window, cx));
        let summary = keybind_summary(
            ("keymap", id),
            input.chords(),
            open,
            warn,
            &theme,
            move |window, cx| toggle(&(), window, cx),
        );

        div().flex_none().child(
            summary
                .child(bounds_probe(self.anchor.clone()))
                .child(self.input.clone()),
        )
    }
}
