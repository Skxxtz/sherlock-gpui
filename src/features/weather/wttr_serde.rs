use gpui::SharedString;
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct WttrResponse {
    pub current_condition: Vec<CurrentCondition>,
    pub weather: Vec<WeatherDay>,
}

#[derive(Deserialize)]
pub(super) struct CurrentCondition {
    #[serde(rename = "temp_F")]
    pub temp_f: String,
    #[serde(rename = "temp_C")]
    pub temp_c: String,
    #[serde(rename = "weatherCode")]
    pub weather_code: String,
    #[serde(rename = "weatherDesc", deserialize_with = "deserialize_value_array")]
    pub weather_desc: SharedString,
    #[serde(rename = "winddirDegree")]
    pub wind_deg: String,
    #[serde(rename = "windspeedMiles")]
    pub wind_miles: String,
    #[serde(rename = "windspeedKmph")]
    pub wind_kmph: String,
}

#[derive(Deserialize)]
pub(super) struct WeatherDay {
    pub astronomy: Vec<Astronomy>,
}

#[derive(Deserialize)]
pub(super) struct Astronomy {
    pub sunrise: String,
    pub sunset: String,
}

fn deserialize_value_array<'de, D>(deserializer: D) -> Result<SharedString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ValueWrapper {
        value: SharedString,
    }
    let arr: Vec<ValueWrapper> = Vec::deserialize(deserializer)?;
    arr.into_iter()
        .next()
        .map(|w| w.value)
        .ok_or_else(|| serde::de::Error::custom("empty weatherDesc array"))
}
