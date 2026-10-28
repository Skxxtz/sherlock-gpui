use std::{cell::Cell, rc::Rc};

use gpui::{
    App,
    Bounds,
    Context,
    EventEmitter,
    FocusHandle,
    Focusable,
    Pixels,
    ScrollHandle,
    SharedString,
    Window,
};

use crate::ui::views::Propagation;

mod actions;
mod render;
#[cfg(test)]
mod tests;

/// Options shown before the list scrolls.
pub const VISIBLE: usize = 6;
const ROW_H: f32 = 30.;
const PAD: f32 = 4.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectEvent {
    /// Another option was picked.
    Changed(usize),
}

pub struct Select {
    options: Vec<SharedString>,
    selected: usize,
    open: bool,
    /// The highlighted option while open (keyboard and mouse).
    cursor: usize,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// The control's bounds: clicking it toggles, so it doesn't count as "outside".
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl EventEmitter<SelectEvent> for Select {}

impl Focusable for Select {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Select {
    pub fn new(options: Vec<SharedString>, selected: usize, cx: &mut Context<Self>) -> Self {
        let selected = selected.min(options.len().saturating_sub(1));
        Self {
            options,
            selected,
            open: false,
            cursor: selected,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            anchor: Rc::default(),
        }
    }

    #[allow(dead_code)] // component API
    #[inline(always)]
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Picks `index` without emitting.
    #[allow(dead_code)] // component API
    pub fn set_selected(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.options.len().saturating_sub(1));
        if index != self.selected {
            self.selected = index;
            cx.notify();
        }
    }

    /// Whether the menu is shown
    #[allow(dead_code)] // component API
    #[inline(always)]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the list on the current option and takes the keyboard.
    pub fn open(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.options.is_empty() {
            return;
        }
        self.open = true;
        self.set_cursor(self.selected);
        self.focus.focus(win, cx);
        cx.notify();
    }

    /// Closes the list without a change and gives focus back.
    #[inline]
    pub fn close(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if self.focus.is_focused(win) {
            win.blur();
        }
        cx.notify();
    }

    /// Toggles menu open state.
    #[inline(always)]
    pub fn toggle(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(win, cx);
        } else {
            self.open(win, cx);
        }
    }

    /// Picks `index`.
    pub fn choose(&mut self, index: usize, _win: &mut Window, cx: &mut Context<Self>) {
        if index < self.options.len() && index != self.selected {
            self.selected = index;
            cx.emit(SelectEvent::Changed(index));
        }
    }

    /// Move the cursor to `index`
    #[inline(always)]
    fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(self.options.len().saturating_sub(1));
        self.scroll.scroll_to_item(self.cursor);
    }

    /// Move the cursor to `index` and notify + propagate
    #[inline]
    fn move_cursor(&mut self, index: usize, cx: &mut Context<Self>) -> Propagation {
        self.set_cursor(index);
        cx.notify();
        Propagation::Stop
    }

    /// The next option after the highlight whose label starts with `c`, wrapping around.
    fn find_next(&self, c: char) -> Option<usize> {
        let n = self.options.len();
        (1..=n).map(|step| (self.cursor + step) % n).find(|&i| {
            self.options[i]
                .chars()
                .next()
                .is_some_and(|first| first.to_lowercase().eq(c.to_lowercase()))
        })
    }
}
