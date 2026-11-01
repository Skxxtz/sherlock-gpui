use std::convert::Infallible;

use chrono::NaiveTime;
use pluck::Extract;

use super::launcher::WeatherClass;

/// The parts of a wttr.in `?format=j2` response we use.
#[derive(Extract)]
pub(super) struct WttrResponse {
    #[at("current_condition")]
    pub current: CurrentCondition,
    #[at("weather.0.astronomy")]
    pub astronomy: Astronomy,
}

#[derive(Extract)]
pub(super) struct CurrentCondition {
    #[at("temp_C")]
    pub temp_c: i32,
    #[at("temp_F")]
    pub temp_f: i32,
    #[at("weatherCode", with = weather_class)]
    pub class: WeatherClass,
    #[at("weatherDesc", trim)]
    pub desc: String,
    #[at("winddirDegree")]
    pub wind_deg: f32,
    #[at("windspeedKmph")]
    pub wind_kmph: u16,
    #[at("windspeedMiles")]
    pub wind_mph: u16,
}

#[derive(Extract)]
pub(super) struct Astronomy {
    #[at(with = clock_time)]
    pub sunrise: NaiveTime,
    #[at(with = clock_time)]
    pub sunset: NaiveTime,
}

/// `"07:23 AM"` → 07:23
#[inline(always)]
fn clock_time(raw: String) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(raw.trim(), "%I:%M %p")
}

/// wttr.in weather code → our weather class.
fn weather_class(code: u16) -> Result<WeatherClass, Infallible> {
    Ok(match code {
        113 => WeatherClass::Clear,
        116 => WeatherClass::FewClouds,
        119 | 122 => WeatherClass::ManyClouds,
        143 | 248 | 260 => WeatherClass::Mist,
        176 | 263 | 299 | 305 | 353 | 356 => WeatherClass::Showers,
        179 | 362 | 365 | 374 => WeatherClass::FreezingScatteredRainStorm,
        182 | 185 | 281 | 284 | 311 | 314 | 317 | 350 | 377 => WeatherClass::FreezingScatteredRain,
        200 | 302 | 308 | 359 | 386 | 389 => WeatherClass::Storm,
        227 | 320 => WeatherClass::SnowScatteredDay,
        230 | 329 | 332 | 338 => WeatherClass::SnowStorm,
        323 | 326 | 335 | 368 | 371 | 392 | 395 => WeatherClass::SnowScatteredStorm,
        266 | 293 | 296 => WeatherClass::ShowersScattered,
        _ => WeatherClass::None,
    })
}
