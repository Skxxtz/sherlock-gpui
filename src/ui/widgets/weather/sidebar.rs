use std::{rc::Rc, sync::Arc};

use chrono::Local;
use gpui::{
    InteractiveElement,
    IntoElement,
    ParentElement,
    RenderOnce,
    SharedString,
    StatefulInteractiveElement,
    Styled,
    div,
    prelude::FluentBuilder,
    px,
};

use crate::{
    app::theme::ActiveTheme,
    config::types::{LengthUnit, TemperatureUnit},
    features::weather::{WeatherData, upcoming},
    ui::{
        components::graph::{Graph, GraphKind, Series},
        launcher::sidebar::wrapper::Sidebar,
    },
    utils::websearch::websearch,
};

/// Forecast slots shown in the graphs: 9 × 3 h ≈ the next 24 hours.
const SLOTS: usize = 9;

#[derive(IntoElement)]
pub struct WeatherSidebar {
    data: Rc<WeatherData>,
    fahrenheit: bool,
    imperial: bool,
}

impl WeatherSidebar {
    pub fn new(data: Rc<WeatherData>, temp_unit: TemperatureUnit, length_unit: LengthUnit) -> Self {
        Self {
            data,
            fahrenheit: temp_unit == TemperatureUnit::Fahrenheit,
            imperial: length_unit.is_imperial(),
        }
    }

    fn temp(&self, c: i32, f: i32) -> f64 {
        if self.fahrenheit { f.into() } else { c.into() }
    }

    fn deg(&self, c: i32, f: i32) -> String {
        format!("{}°", self.temp(c, f))
    }
}

/// x-axis label for slot index `v` (ticks between slots stay empty).
fn slot_label(labels: Arc<Vec<String>>) -> impl Fn(f64) -> String + 'static {
    move |v| {
        if v.fract() != 0.0 || v < 0.0 {
            return String::new();
        }
        labels.get(v as usize).cloned().unwrap_or_default()
    }
}

impl RenderOnce for WeatherSidebar {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let d = &self.data;

        let slots = upcoming(&d.forecast, Local::now().naive_local(), SLOTS);
        let hours = slots.iter().map(|(_, h)| *h);
        let labels = Arc::new(
            slots
                .iter()
                .map(|(at, _)| at.format("%H:%M").to_string())
                .collect::<Vec<_>>(),
        );

        let accent = theme.accent;
        let feels_like = theme.companion(theme.accent);

        let temps = Graph::new(GraphKind::Area)
            .id("weather-temps")
            .series(
                Series::from_values(
                    "Temperature",
                    hours.clone().map(|h| self.temp(h.temp_c, h.temp_f)),
                )
                .color(accent),
            )
            .series(
                Series::from_values(
                    "Feels like",
                    hours
                        .clone()
                        .map(|h| self.temp(h.feels_like_c, h.feels_like_f)),
                )
                .color(feels_like),
            )
            .x_format(slot_label(labels.clone()))
            .y_format(|v| format!("{v:.0}°"))
            .height(px(100.));

        let rain = Graph::new(GraphKind::Bar)
            .id("rain-percent")
            .series(Series::from_values("Rain", hours.map(|h| h.chance.rain)).color(feels_like))
            .y_range(0., 100.)
            .ticks(2)
            .x_format(slot_label(labels))
            .y_format(|v| format!("{v:.0}%"))
            .legend(false)
            .height(px(50.));

        let title: SharedString = match &d.area {
            Some(a) => format!("{}, {}", a.name, a.country).into(),
            None => d.location.clone(),
        };
        let today = d.forecast.first();

        Sidebar::new(theme.clone())
            .min_h_full()
            .header(
                d.icon.as_ref(),
                title,
                [d.temperature.clone(), d.condition.clone()],
            )
            .when(!slots.is_empty(), |s| {
                s.section("Next 24 hours")
                    .child(div().mt_3().child(temps))
                    .section("Chance of rain")
                    .child(div().my_3().child(rain))
            })
            .separator()
            .section("Today")
            .when_some(today, |s, t| {
                s.row(
                    "Low / High",
                    format!(
                        "{} / {}",
                        self.deg(t.min_c, t.min_f),
                        self.deg(t.max_c, t.max_f)
                    ),
                )
                .row("Sun hours", format!("{:.0} h", t.sun_hours))
            })
            .row("Sunrise", d.sunrise.format("%H:%M").to_string())
            .row("Sunset", d.sunset.format("%H:%M").to_string())
            .when_some(d.current.as_ref(), |s, c| {
                let (wind, vis) = if self.imperial {
                    (
                        format!("{} mph {}", c.wind_mph, c.wind_dir),
                        format!("{} mi", c.visibility_miles),
                    )
                } else {
                    (
                        format!("{} km/h {}", c.wind_kmph, c.wind_dir),
                        format!("{} km", c.visibility),
                    )
                };
                s.separator()
                    .section("Now")
                    .row("Feels like", self.deg(c.feels_like_c, c.feels_like_f))
                    .row("Humidity", format!("{}%", c.humidity))
                    .row("Wind", wind)
                    .row("Pressure", format!("{} hPa", c.pressure))
                    .row("Visibility", vis)
                    .row("UV index", c.uv_index.to_string())
            })
            .child(
                div()
                    .mt_10()
                    .w_full()
                    .flex()
                    .justify_center()
                    .text_color(theme.tertiary_text)
                    .text_xs()
                    .child("All data is provided by ")
                    .child(
                        div()
                            .id("weather-link")
                            .underline()
                            .text_color(theme.accent)
                            .child("wttr.in")
                            .cursor_pointer()
                            .on_click(|_, _, _| {
                                let _ = websearch("plain", "wttr.in", None, &[]);
                            }),
                    ),
            )
    }
}
