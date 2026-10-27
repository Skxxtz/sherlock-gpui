use gpui::Hsla;

mod data;
mod input;
mod item;
mod render;
mod state;

pub use item::ColorItem;

const PANEL_W: f32 = 256.;
const PAD: f32 = 12.;
/// Width of the field and the hue slider.
const INNER_W: f32 = PANEL_W - 2. * PAD;
const FIELD_H: f32 = 140.;
const HUE_H: f32 = 12.;
const HANDLE: f32 = 14.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorEvent {
    /// The picked color changed (live, while dragging too).
    Changed(Hsla),
}
