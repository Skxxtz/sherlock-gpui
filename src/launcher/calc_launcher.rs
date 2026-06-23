use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry, capabilities_section},
    launcher::{LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, calculator::CalcData},
    utils::{errors::SherlockMessage, intent::Capabilities},
    variant_name,
};
use indoc::indoc;
use serde_json::Value;
use std::sync::OnceLock;

mod currency;
mod trading_view_api;

pub use currency::Currency;

pub static CURRENCIES: OnceLock<Option<Currency>> = OnceLock::new();

/// The following arguments are available to users:
/// - `currency_update_interval`
/// - `capabilities`
#[derive(Clone, Debug)]
pub struct CalculatorLauncher {}

impl LauncherProvider for CalculatorLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        // initialize currencies
        let update_interval = raw
            .args
            .get("currency_update_interval")
            .and_then(|interval| interval.as_u64())
            .unwrap_or(1440);

        #[cfg(not(test))]
        spawn_currency_update(update_interval);

        Ok(LauncherType::Calculator(CalculatorLauncher {}))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let capabilities: Vec<String> = match opts.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let caps = Capabilities::from_strings(&capabilities);
        let inner = CalcData::new(caps);

        Ok(vec![RenderableChild::Calc { launcher, inner }])
    }
}

#[cfg(not(test))]
fn spawn_currency_update(update_interval: u64) {
    tokio::spawn(async move {
        match Currency::get_exchange(update_interval).await {
            Ok(r) => {
                let _result = CURRENCIES.set(Some(r));
            }
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    });
}

// DOCS
impl LauncherDoc for CalculatorLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(CalculatorLauncher),
            variant_name: variant_name!(Calculator),
            description: "Allows math calculations and different unit conversions.",
            args: &[
                FieldDoc {
                    name: "capabilities",
                    ty: "Capability[]",
                    required: false,
                    default: Some(r#"[ "calc.units", "calc.math" ]"#),
                    description: "The capabilities the calculator should have.",
                },
                FieldDoc {
                    name: "currency_update_interval",
                    ty: "u64",
                    required: false,
                    default: Some("1440"),
                    description: "Number of minutes to keep the currency cache alive.",
                },
            ],
            args_explanations: &[capabilities_section],
            examples: &[Example {
                description: "Basic calculator config",
                json: indoc! {
                    r#" {
                        "name": "Calculator",
                        "type": "calculator",
                        "alias": "calc",
                        "args": {
                            "currency_update_interval": 60,
                            "capabilities": [
                                "calc.math",
                                "calc.units",
                                "calc.currencies",
                                "colors"
                            ]
                        },
                        "priority": 1,
                        "on_return": "copy"
                    } "#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}
