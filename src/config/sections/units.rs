use serde::{Deserialize, Serialize};

use super::Section;
use crate::config::types::{LengthUnit, TemperatureUnit, Text, VolumeUnit, WeightUnit};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct ConfigUnits {
    pub lengths: LengthUnit,
    pub weights: WeightUnit,
    pub volumes: VolumeUnit,
    pub temperatures: TemperatureUnit,
    /// An ISO 4217 code, lowercase (`"eur"`).
    pub currency: Text,
}

impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: LengthUnit::Meters,
            weights: WeightUnit::Kilograms,
            volumes: VolumeUnit::Liters,
            temperatures: TemperatureUnit::Celsius,
            currency: Text::new_static("eur"),
        }
    }
}

impl Section for ConfigUnits {
    fn validate(&mut self, warn: &mut Vec<String>) {
        let code = self.currency.trim().to_ascii_lowercase();
        if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
            warn.push(format!(
                "`units.currency` = {:?} is not a currency code, using \"eur\"",
                &*self.currency
            ));
            self.currency = Text::new_static("eur");
        } else if *self.currency != code {
            self.currency = code.into();
        }
    }
}
