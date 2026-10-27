use std::{cell::Cell, rc::Rc};

use gpui::{
    AppContext, Bounds, Context, Entity, EventEmitter, Hsla, ParentElement, Pixels, Render, Styled,
    Subscription, Window, div,
};

use crate::{
    app::theme::ActiveTheme,
    ui::components::{color_swatch::color_swatch, keybind::bounds_probe},
};

use super::ColorEvent;
use super::input::ColorInput;

pub struct ColorItem {
    input: Entity<ColorInput>,
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    _subs: [Subscription; 2],
}

impl EventEmitter<ColorEvent> for ColorItem {}

impl ColorItem {
    /// Create new `ColorInput` instance.
    pub fn new(color: Hsla, cx: &mut Context<Self>) -> Self {
        let anchor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        let input = cx.new(|cx| ColorInput::new(color, anchor.clone(), cx));
        let _subs = [
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.subscribe(&input, |_, _, event: &ColorEvent, cx| cx.emit(*event)),
        ];
        Self {
            input,
            anchor,
            _subs,
        }
    }

    /// The picker this item opens.
    #[allow(dead_code)]
    #[inline(always)]
    pub fn input(&self) -> &Entity<ColorInput> {
        &self.input
    }

    /// Replaces the color without emitting.
    #[allow(dead_code)]
    #[inline(always)]
    pub fn set_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.set_color(color, cx));
    }

    /// Opens the color picker
    #[inline(always)]
    pub fn open(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.open(win, cx))
    }

    /// Toggles the color picker
    #[inline(always)]
    pub fn toggle(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.toggle(win, cx));
    }
}

impl Render for ColorItem {
    fn render(
        &mut self,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::prelude::IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let id = cx.entity_id().as_u64() as usize;
        let color = self.input.read(cx).color();
        let toggle = cx.listener(|this, _: &(), win, cx| this.toggle(win, cx));

        div()
            .relative()
            .child(color_swatch(
                ("color", id),
                color,
                &theme,
                move |_, win, cx| toggle(&(), win, cx),
            ))
            .child(bounds_probe(self.anchor.clone()))
            .child(self.input.clone())
    }
}
