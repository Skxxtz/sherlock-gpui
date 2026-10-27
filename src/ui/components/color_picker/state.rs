use gpui::Hsla;
use std::{cell::Cell, rc::Rc};

use super::data::Hsv;
use super::render::Drag;

/// An open picker. Owned by the view showing it; dropped when the picker closes.
#[derive(Clone)]
pub struct ColorPickerState {
    /// The color the picker was opened with (Esc and the "before" swatch return to it).
    pub original: Hsla,
    pub hsv: Hsv,
    pub drag: Rc<Cell<Option<Drag>>>,
}

impl ColorPickerState {
    pub fn new(color: Hsla) -> Self {
        Self {
            original: color,
            hsv: Hsv::from_hsla(color),
            drag: Rc::default(),
        }
    }

    pub fn color(&self) -> Hsla {
        self.hsv.to_hsla()
    }

    pub fn revert(&mut self) {
        self.hsv = Hsv::from_hsla(self.original);
    }

    /// Hue by `delta` turns, wrapping.
    pub fn nudge_hue(&mut self, delta: f32) {
        self.hsv.h = (self.hsv.h + delta).rem_euclid(1.);
    }

    /// Brightness by `delta`, clamped.
    pub fn nudge_value(&mut self, delta: f32) {
        self.hsv.v = (self.hsv.v + delta).clamp(0., 1.);
    }
}
