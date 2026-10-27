use gpui::{Hsla, hsla};

/// Hue, saturation, value; all in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn from_hsla(c: Hsla) -> Self {
        let v = c.l + c.s * c.l.min(1. - c.l);
        let s = if v <= 0. { 0. } else { 2. * (1. - c.l / v) };
        Self { h: c.h, s, v }
    }

    pub fn to_hsla(self) -> Hsla {
        let l = self.v * (1. - self.s / 2.);
        let s = if l <= 0. || l >= 1. {
            0.
        } else {
            (self.v - l) / l.min(1. - l)
        };
        hsla(self.h, s, l, 1.)
    }
}
