use std::path::PathBuf;

use paste::paste;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::EnumIter;
use strum_macros::EnumString;

use super::trading_view_api::{CurrencyItem, TradingViewApiRequest, TradingViewApiResponse};
use crate::utils::{cache::JsonCache, errors::SherlockMessage, files::home_dir_owned};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Course {
    Valid(f32),
    #[default]
    Invalid,
}

impl Serialize for Course {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Valid(f) => serializer.serialize_f32(*f),
            Self::Invalid => serializer.serialize_f32(0.0),
        }
    }
}

impl<'de> Deserialize<'de> for Course {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let f = f32::deserialize(deserializer)?;
        Ok(if f == 0.0 {
            Self::Invalid
        } else {
            Self::Valid(f)
        })
    }
}

impl From<f32> for Course {
    fn from(f: f32) -> Self {
        if f == 0.0 {
            Self::Invalid
        } else {
            Self::Valid(f)
        }
    }
}

macro_rules! currency_struct {
    ($($iso:ident),*) => {
        paste! {
            #[derive(Clone, Debug, Deserialize, Serialize)]
            #[serde(rename_all = "lowercase")]
            pub struct Currency {
                pub usd: Course,
                $(pub $iso: Course,)*
            }

            impl Default for Currency {
                fn default() -> Self {
                    Self {
                        usd: Course::Valid(1.0),
                        $($iso: Course::Invalid,)*
                    }
                }
            }

            #[derive(Debug, EnumIter, EnumString, PartialEq)]
            pub enum CurrencyCode {
                $( [< $iso:camel >], )*
            }

            impl<'de> serde::Deserialize<'de> for CurrencyCode {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    struct CurrencyVisitor;

                    impl<'de> serde::de::Visitor<'de> for CurrencyVisitor {
                        type Value = CurrencyCode;

                        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                            formatter.write_str("a currency code string")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match value {
                                $(
                                    concat!(stringify!([< $iso:upper >]), "USD") => Ok(CurrencyCode::[< $iso:camel >]),
                                )*
                                _ => Err(serde::de::Error::unknown_variant(value, &["valid currency codes"])),
                            }
                        }
                    }

                    deserializer.deserialize_str(CurrencyVisitor)
                }
            }

            impl Currency {
                pub(super) fn set(&mut self, item: CurrencyItem) {
                    match item.space {
                        $( CurrencyCode::[< $iso:camel >] => self.$iso = item.factor.into(), )*
                    }
                }
                pub(super) fn iso_codes() -> &'static [&'static str] {
                    &[$( stringify!([< $iso:upper >]), )* ]
                }
            }
        }
    };
}
currency_struct!(
    eur, // Euro
    jpy, // Japanese Yen
    gbp, // British Pound Sterling
    aud, // Australian Dollar
    cad, // Canadian Dollar
    chf, // Swiss Franc
    cny, // Chinese Yuan
    nzd, // New Zealand Dollar
    sek, // Swedish Krona
    nok, // Norwegian Krona
    mxn, // Mexican Peso
    sgd, // Singapore Dollar
    hkd, // Hong Kong Dollar
    krw, // South Korean Won
    pln, // Polish złoty
    pen  // Peruvian Sole
);

// ---------- NORMAL IMPL ----------

impl From<TradingViewApiResponse> for Currency {
    fn from(value: TradingViewApiResponse) -> Self {
        let mut currency = Self::default();
        value.currency_items().for_each(|item| currency.set(item));
        currency
    }
}

impl JsonCache for Currency {
    fn cache_path() -> std::path::PathBuf {
        home_dir_owned()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".cache/sherlock/currency/currency.json")
    }
}

impl Currency {
    pub async fn get_exchange(update_interval: u64) -> Result<Currency, SherlockMessage> {
        if let Ok(Some(curr)) = Currency::read_from_cache(update_interval) {
            return Ok(curr);
        }

        // Fetch new data
        let currency_response: TradingViewApiResponse =
            TradingViewApiRequest::fetch_exchange_data().await?;
        let currency = Self::from(currency_response);
        currency.write_to_cache()?;

        Ok(currency)
    }
}
