use std::time::Duration;

use gpui::SharedString;
use smallvec::SmallVec;

use crate::{
    launcher::calc_launcher::CURRENCIES,
    utils::{
        intent::{
            colors::ColorConverter,
            cursor::Cursor,
            parsers::{color::ColorParser, translation::TranslationParser, units::UnitParser},
            translation::Language,
            units::{Unit, UnitCategory},
        },
        url_detection::is_url,
    },
};

mod colors;
pub mod cursor;
pub mod parsers;
pub mod translation;
mod units;
mod utils;

pub use units::Capabilities;
#[cfg(feature = "docs")]
pub use units::docs::CAPABILITY_DOCS;
pub use utils::IntentResult;

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    ColorConvert {
        from_space: &'static str,
        values: SmallVec<[f32; 4]>,
        to_space: &'static str,
    },
    ColorDisplay {
        from_space: &'static str,
        values: SmallVec<[f32; 4]>,
    },
    Conversion {
        value: f64,
        from: Unit,
        to: Unit,
    },
    Url {
        url: SharedString,
    },
    Translation {
        text: SharedString,
        target_lang: Language,
    },
    Timer {
        duration: Duration,
    },
    None,
}

impl Intent {
    pub fn execute(&self) -> Option<IntentResult> {
        match self {
            Intent::Conversion { value, from, to } => {
                // early return on domain mismatch
                if from.category() != to.category() {
                    return None;
                }

                if from.category() == UnitCategory::Currency && CURRENCIES.get().is_none() {
                    return Some(IntentResult::String(
                        "Loading exchange rates...".to_string().into(),
                    ));
                }

                // handle temperature (non-linear)
                if from.category() == UnitCategory::Temperature {
                    let result = match (from, to) {
                        (Unit::Celsius, Unit::Fahrenheit) => (value * 9.0 / 5.0) + 32.0,
                        (Unit::Fahrenheit, Unit::Celsius) => (value - 32.0) * 5.0 / 9.0,
                        _ => *value,
                    };
                    return Some(IntentResult::String(
                        format!("{:.1} {}", result, to.symbol()).into(),
                    ));
                }

                // handle linear
                // Formula: y = val * (from_factor / to_factor)
                let result = value * (from.factor() / to.factor());

                Some(IntentResult::String(self.format_result(result, to).into()))
            }
            Intent::ColorDisplay { from_space, values } => {
                ColorConverter::normalize(from_space, values).map(IntentResult::Color)
            }
            Intent::ColorConvert {
                from_space,
                values,
                to_space,
            } => ColorConverter::convert(from_space, values, to_space)
                .map(|r| IntentResult::String(r.into())),
            _ => None,
        }
    }

    fn format_result(&self, result: f64, unit: &Unit) -> String {
        // Smart formatting based on magnitude
        let formatted = if result == 0.0 {
            "0".to_string()
        } else if result.abs() < 0.001 || result.abs() >= 1_000_000_000.0 {
            format!("{:.4e}", result) // Scientific notation for extreme sizes
        } else if result.fract() == 0.0 {
            format!("{:.0}", result) // No decimals if it's an integer
        } else {
            format!("{:.2}", result) // Standard 2 decimals
        };

        format!("{} {}", formatted, unit.symbol())
    }
}

impl Intent {
    pub fn parse(input: &str, caps: &Capabilities) -> Intent {
        let raw = input.trim();
        if raw.is_empty() {
            return Intent::None;
        }

        let clean: SmallVec<[&str; 16]> = Self::tokenize_kill_noise(raw).take(16).collect();
        let cur = Cursor::new(&clean);

        if caps.allows(Capabilities::COLORS)
            && let Some(intent) = ColorParser::parse_intent(cur)
        {
            return intent;
        }
        if let Some(intent) = UnitParser::parse_intent(cur, caps) {
            return intent;
        }
        if let Some(intent) = TranslationParser::parse_intent(raw) {
            return intent;
        }
        if let Some(intent) = Self::parse_url(raw) {
            return intent;
        }

        Intent::None
    }

    fn parse_url(input: &str) -> Option<Intent> {
        is_url(input).then_some(Intent::Url {
            url: input.to_string().into(),
        })
    }
}
