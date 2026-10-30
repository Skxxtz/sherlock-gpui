use gpui::{IntoElement, Pixels};

use super::pixel_art::{Grid, pixel_art};
use crate::features::weather::launcher::WeatherClass;

mod debug;
mod icons;
mod palette;
mod sprites;

#[allow(unused_imports)]
pub use debug::weather_icon_grid;
use icons::*;

/// Weather icon at roughly `size` (snapped down to a whole multiple of 15).
pub fn weather_icon(class: &WeatherClass, size: Pixels) -> impl IntoElement {
    pixel_art(class.icon(), size, palette::weather_palette)
}

impl WeatherClass {
    pub fn icon(&self) -> &'static Grid<ICON_SIZE> {
        match self {
            Self::Clear => &CLEAR,
            Self::FewClouds => &FEW_CLOUDS,
            Self::ManyClouds => &MANY_CLOUDS,
            Self::Mist => &MIST,
            Self::Showers => &SHOWERS,
            Self::ShowersScattered => &SHOWERS_SCATTERED,
            Self::Storm => &STORM,
            Self::FreezingScatteredRain => &FREEZING_RAIN,
            Self::FreezingScatteredRainStorm => &FREEZING_RAIN_STORM,
            Self::SnowScatteredDay => &SNOW_SCATTERED_DAY,
            Self::SnowScatteredStorm => &SNOW_SCATTERED_STORM,
            Self::SnowStorm => &SNOW_STORM,
            Self::None => &UNKNOWN,
        }
    }
}
