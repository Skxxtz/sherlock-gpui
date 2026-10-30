use gpui::{IntoElement, ParentElement, Pixels, Styled, div, px, rgb};

use super::weather_icon;
use crate::features::weather::launcher::WeatherClass;

#[allow(unused)]
const ALL_WEATHER: [(&str, WeatherClass); 13] = [
    ("Clear", WeatherClass::Clear),
    ("FewClouds", WeatherClass::FewClouds),
    ("ManyClouds", WeatherClass::ManyClouds),
    ("Mist", WeatherClass::Mist),
    ("Showers", WeatherClass::Showers),
    ("ShowersScattered", WeatherClass::ShowersScattered),
    ("Storm", WeatherClass::Storm),
    ("FreezingScatteredRain", WeatherClass::FreezingScatteredRain),
    (
        "FreezingScatteredRainStorm",
        WeatherClass::FreezingScatteredRainStorm,
    ),
    ("SnowScatteredDay", WeatherClass::SnowScatteredDay),
    ("SnowScatteredStorm", WeatherClass::SnowScatteredStorm),
    ("SnowStorm", WeatherClass::SnowStorm),
    ("None", WeatherClass::None),
];

/// All weather icons side by side, wrapping, each with its name underneath.
#[allow(unused)]
pub fn weather_icon_grid(size: Pixels) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap_4()
        .p_4()
        .children(ALL_WEATHER.iter().map(|(name, class)| {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .w(size + px(32.)) // fixed cell width so long names don't shift the grid
                .child(weather_icon(class, size))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xa0a0a0))
                        .text_center()
                        .child(*name),
                )
        }))
}
