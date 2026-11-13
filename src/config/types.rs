#![allow(unused_imports)]

use std::sync::Mutex;

mod cfg_path;
mod enums;
mod margins;
mod percent;
mod text;

pub use cfg_path::CfgPath;
pub use enums::{Anchor, ConfigEnum, Edge, LengthUnit, TemperatureUnit, VolumeUnit, WeightUnit};
pub use margins::Margins;
pub use percent::Percent;
pub use text::{Text, non_empty};

static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Records a problem found while reading the config.
pub fn warn(msg: impl Into<String>) {
    if let Ok(mut w) = WARNINGS.lock() {
        w.push(msg.into())
    }
}

/// Takes the problems recorded since the last call.
pub fn take_warnings() -> Vec<String> {
    WARNINGS
        .lock()
        .map(|mut w| std::mem::take(&mut *w))
        .unwrap_or_default()
}
