use super::{
    palette::{BACK, DARK},
    sprites::*,
};
use crate::ui::components::pixel_art::{Grid, Layer, Sprite, at, compose};

/// Icon size in pixels (rows × columns). Odd, so there is a centre pixel (7, 7).
pub const ICON_SIZE: usize = 15;

/// Cloud above precipitation.
const TOP_CLOUD: Layer = at(CLOUD, 1, 1);
/// Precipitation below `TOP_CLOUD`.
const fn precip(sprite: Sprite) -> Layer {
    at(sprite, 1, 9)
}
/// Sun peeking out behind a cloud, with precipitation below.
const PEEK_SUN: Layer = at(SUN_SMALL, 0, 0);
const PEEK_CLOUD: Layer = at(CLOUD, 2, 3);
const fn peek_precip(sprite: Sprite) -> Layer {
    at(sprite, 1, 10)
}

pub(super) const CLEAR: Grid<ICON_SIZE> = compose(&[at(SUN, 1, 1)]);

pub const FEW_CLOUDS: Grid<ICON_SIZE> = compose(&[at(SUN_SMALL, 0, 1), at(CLOUD, 2, 6)]);

pub const MANY_CLOUDS: Grid<ICON_SIZE> = compose(&[at(CLOUD, 2, 2).swap(BACK), at(CLOUD, 0, 6)]);

pub const MIST: Grid<ICON_SIZE> = compose(&[at(MIST_LINES, 0, 3)]);

pub const SHOWERS: Grid<ICON_SIZE> = compose(&[TOP_CLOUD, precip(RAIN)]);

pub const SHOWERS_SCATTERED: Grid<ICON_SIZE> =
    compose(&[PEEK_SUN, PEEK_CLOUD, peek_precip(RAIN_LIGHT)]);

pub const STORM: Grid<ICON_SIZE> = compose(&[TOP_CLOUD.swap(DARK), precip(RAIN), at(BOLT, 4, 8)]);

pub const FREEZING_RAIN: Grid<ICON_SIZE> = compose(&[TOP_CLOUD, precip(SLEET)]);

pub const FREEZING_RAIN_STORM: Grid<ICON_SIZE> = compose(&[TOP_CLOUD.swap(DARK), precip(SLEET)]);

pub const SNOW_SCATTERED_DAY: Grid<ICON_SIZE> =
    compose(&[PEEK_SUN, PEEK_CLOUD, peek_precip(FLAKES)]);

pub const SNOW_SCATTERED_STORM: Grid<ICON_SIZE> = compose(&[TOP_CLOUD, precip(FLAKES)]);

pub const SNOW_STORM: Grid<ICON_SIZE> = compose(&[TOP_CLOUD.swap(DARK), precip(SNOW)]);

pub const UNKNOWN: Grid<ICON_SIZE> = compose(&[at(QUESTION, 4, 3)]);
