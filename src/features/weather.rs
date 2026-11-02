mod display;
pub mod launcher;
mod wttr_serde;

pub use launcher::{WeatherData, WeatherLauncher};
pub use wttr_serde::upcoming;
