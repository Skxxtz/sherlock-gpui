use super::{
    launcher::{WeatherClass, WeatherData, WeatherIconTheme, WeatherLauncher},
    wttr_serde::{CurrentCondition, WttrResponse},
};
use crate::{
    config::{
        ConfigGuard,
        types::{LengthUnit, TemperatureUnit},
    },
    launcher::utils::to_title_case,
    loader::{IconType, resolve_icon_path},
    sherlock_msg,
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

fn format_temp(current: &CurrentCondition, unit: TemperatureUnit) -> String {
    match unit {
        TemperatureUnit::Fahrenheit => format!("{}°F", current.temp_f),
        TemperatureUnit::Celsius => format!("{}°C", current.temp_c),
    }
}

fn format_wind(current: &CurrentCondition, length_unit: LengthUnit) -> String {
    let direction = get_wind_dir(current.wind_deg);

    if length_unit.is_imperial() {
        format!("{} {}mph", direction, current.wind_mph)
    } else {
        format!("{} {}km/h", direction, current.wind_kmph)
    }
}

fn resolve_icon(theme: &WeatherIconTheme, class: &WeatherClass) -> Option<IconType> {
    let path = if matches!(theme, WeatherIconTheme::Sherlock) {
        format!("weather-icons/sherlock-weather-{class}")
    } else {
        format!("weather-{class}")
    };

    resolve_icon_path(&path)
}

pub(super) fn transform_weather(
    raw: WttrResponse,
    launcher: &WeatherLauncher,
) -> Result<WeatherData, SherlockMessage> {
    let WttrResponse {
        current,
        area,
        days,
    } = raw;
    let today = days.first().ok_or_else(|| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DeserializationError("Weather Data".into()),
            "wttr.in response has no forecast (use ?format=j1)"
        )
    })?;
    let (sunrise, sunset) = (today.astronomy.sunrise, today.astronomy.sunset);
    let config = ConfigGuard::read()?;

    Ok(WeatherData {
        temperature: format_temp(&current, config.units.temperatures).into(),
        icon: resolve_icon(&launcher.icon_theme, &current.class),
        format_str: format!(
            "{}  {}",
            to_title_case(&launcher.location),
            format_wind(&current, config.units.lengths)
        )
        .into(),
        condition: current.desc.clone().into(),
        location: launcher.location.clone().into(),
        css: current.class.clone(),
        sunset,
        sunrise,
        forecast: days,
        area,
        current: Some(current),
        init: true,
    })
}

fn get_wind_dir(deg: f32) -> &'static str {
    // 360 / 8 segments = 45 degrees per segment
    // We add 22.5 to center the segments (e.g., N is 337.5 to 22.5)
    let win_dirs = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
    let index = ((deg + 22.5) / 45.0).floor() as usize % 8;
    win_dirs[index]
}
