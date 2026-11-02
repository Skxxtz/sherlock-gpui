use std::convert::Infallible;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use pluck::Extract;
use serde::{Deserialize, Serialize};

use super::launcher::WeatherClass;

#[derive(Extract, Debug, Clone)]
pub struct WttrResponse {
    #[at("current_condition")]
    pub current: CurrentCondition,
    #[at("nearest_area")]
    pub area: Option<Area>,
    /// Today and the next two days. Empty for `?format=j2`, which has no forecast.
    #[at("weather.*")]
    pub days: Vec<Day>,
}

#[derive(Extract, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CurrentCondition {
    /// Observation time in **UTC**.
    #[at("observation_time", with = clock_time)]
    pub observed_utc: NaiveTime,
    #[at("temp_C")]
    pub temp_c: i32,
    #[at("temp_F")]
    pub temp_f: i32,
    #[at("FeelsLikeC")]
    pub feels_like_c: i32,
    #[at("FeelsLikeF")]
    pub feels_like_f: i32,
    #[at("weatherCode", with = weather_class)]
    pub class: WeatherClass,
    #[at("weatherDesc", trim)]
    pub desc: String,
    #[at("cloudcover")]
    pub cloud_cover: u8,
    pub humidity: u8,
    #[at("precipMM")]
    pub precip_mm: f32,
    #[at("precipInches")]
    pub precip_in: f32,
    /// hPa
    pub pressure: u16,
    #[at("pressureInches")]
    pub pressure_in: u16,
    /// km
    pub visibility: u16,
    #[at("visibilityMiles")]
    pub visibility_miles: u16,
    #[at("uvIndex")]
    pub uv_index: u8,
    #[at("winddirDegree")]
    pub wind_deg: f32,
    #[at("winddir16Point")]
    pub wind_dir: String,
    #[at("windspeedKmph")]
    pub wind_kmph: u16,
    #[at("windspeedMiles")]
    pub wind_mph: u16,
}

#[derive(Extract, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Area {
    #[at("areaName")]
    pub name: String,
    pub country: String,
    pub region: String,
    pub latitude: f64,
    pub longitude: f64,
    #[at(default)]
    pub population: u64,
}

#[derive(Extract, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Day {
    #[at(with = iso_date)]
    pub date: NaiveDate,
    pub astronomy: Astronomy,
    #[at("maxtempC")]
    pub max_c: i32,
    #[at("maxtempF")]
    pub max_f: i32,
    #[at("mintempC")]
    pub min_c: i32,
    #[at("mintempF")]
    pub min_f: i32,
    #[at("avgtempC")]
    pub avg_c: i32,
    #[at("avgtempF")]
    pub avg_f: i32,
    #[at("sunHour")]
    pub sun_hours: f32,
    #[at("totalSnow_cm")]
    pub total_snow_cm: f32,
    #[at("uvIndex")]
    pub uv_index: u8,
    /// 3-hour steps: 00:00, 03:00, … 21:00.
    #[at("hourly.*")]
    pub hourly: Vec<Hour>,
}

#[derive(Extract, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Astronomy {
    #[at(with = clock_time)]
    pub sunrise: NaiveTime,
    #[at(with = clock_time)]
    pub sunset: NaiveTime,
    #[at(none_if = "No moonrise", with = clock_time)]
    pub moonrise: Option<NaiveTime>,
    #[at(none_if = "No moonset", with = clock_time)]
    pub moonset: Option<NaiveTime>,
    #[at(trim)]
    pub moon_phase: String,
    /// Percent
    pub moon_illumination: u8,
}

#[derive(Extract, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Hour {
    #[at(with = hhmm)]
    pub time: NaiveTime,

    #[at("tempC")]
    pub temp_c: i32,
    #[at("tempF")]
    pub temp_f: i32,
    #[at("FeelsLikeC")]
    pub feels_like_c: i32,
    #[at("FeelsLikeF")]
    pub feels_like_f: i32,
    #[at("DewPointC")]
    pub dew_point_c: i32,
    #[at("DewPointF")]
    pub dew_point_f: i32,
    #[at("HeatIndexC")]
    pub heat_index_c: i32,
    #[at("HeatIndexF")]
    pub heat_index_f: i32,
    #[at("WindChillC")]
    pub wind_chill_c: i32,
    #[at("WindChillF")]
    pub wind_chill_f: i32,

    #[at("weatherCode", with = weather_class)]
    pub class: WeatherClass,
    #[at("weatherDesc", trim)]
    pub desc: String,

    /// Chances in percent.
    #[at("")]
    pub chance: Chances,

    #[at("cloudcover")]
    pub cloud_cover: u8,
    pub humidity: u8,
    #[at("precipMM")]
    pub precip_mm: f32,
    #[at("precipInches")]
    pub precip_in: f32,
    /// hPa
    pub pressure: u16,
    #[at("pressureInches")]
    pub pressure_in: u16,
    /// km
    pub visibility: u16,
    #[at("visibilityMiles")]
    pub visibility_miles: u16,
    #[at("uvIndex")]
    pub uv_index: u8,
    /// Solar radiation, W/m²
    #[at("diffRad", default)]
    pub diffuse_rad: f32,
    #[at("shortRad", default)]
    pub shortwave_rad: f32,

    #[at("winddirDegree")]
    pub wind_deg: f32,
    #[at("winddir16Point")]
    pub wind_dir: String,
    #[at("windspeedKmph")]
    pub wind_kmph: u16,
    #[at("windspeedMiles")]
    pub wind_mph: u16,
    #[at("WindGustKmph")]
    pub gust_kmph: u16,
    #[at("WindGustMiles")]
    pub gust_mph: u16,
}

/// The `chanceof*` fields of an hour, in percent.
#[derive(Extract, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Chances {
    #[at("chanceofrain")]
    pub rain: u8,
    #[at("chanceofsnow")]
    pub snow: u8,
    #[at("chanceofthunder")]
    pub thunder: u8,
    #[at("chanceoffog")]
    pub fog: u8,
    #[at("chanceoffrost")]
    pub frost: u8,
    #[at("chanceofovercast")]
    pub overcast: u8,
    #[at("chanceofsunshine")]
    pub sunshine: u8,
    #[at("chanceofwindy")]
    pub windy: u8,
    #[at("chanceofhightemp")]
    pub high_temp: u8,
    /// Chance of staying dry.
    #[at("chanceofremdry")]
    pub dry: u8,
}

/// The next `n` forecast slots, starting with the one `now` falls into.
///
/// wttr.in gives 8 slots per day (00:00, 03:00 … 21:00, local to the
/// location); a slot covers the 3 hours from its start time.
pub fn upcoming(days: &[Day], now: NaiveDateTime, n: usize) -> Vec<(NaiveDateTime, &Hour)> {
    days.iter()
        .flat_map(|d| d.hourly.iter().map(move |h| (d.date.and_time(h.time), h)))
        .filter(|(at, _)| *at + TimeDelta::hours(3) > now)
        .take(n)
        .collect()
}

/// `"07:23 AM"` → 07:23
fn clock_time(raw: String) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(raw.trim(), "%I:%M %p")
}

/// `"900"` → 09:00
fn hhmm(raw: u16) -> Result<NaiveTime, String> {
    NaiveTime::from_hms_opt((raw / 100).into(), (raw % 100).into(), 0)
        .ok_or_else(|| format!("{raw} is not a time"))
}

/// `"2026-11-01"`
fn iso_date(raw: String) -> Result<NaiveDate, chrono::ParseError> {
    raw.trim().parse()
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
