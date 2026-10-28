use gpui::SharedString;

use super::model::SettingValue;

/// What the struct can't know about a field: the allowed range, or the allowed strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Meta {
    None,
    /// For numbers: `min..=max` in `step`s. Floats are shown in percent (`0.6` ↔ `60`).
    Range {
        min: i64,
        max: i64,
        step: i64,
    },
    /// For strings with a fixed set of values.
    #[allow(dead_code)] // no config string has a fixed set of values yet
    Choices(&'static [&'static str]),
}

impl Meta {
    #[allow(dead_code)]
    pub(crate) const fn range(min: i64, max: i64) -> Self {
        Self::Range { min, max, step: 1 }
    }

    pub(crate) const fn range_step(min: i64, max: i64, step: i64) -> Self {
        Self::Range { min, max, step }
    }

    #[allow(dead_code)] // see `Choices`
    pub(crate) const fn choices(options: &'static [&'static str]) -> Self {
        Self::Choices(options)
    }

    /// The number control for `value` within this range (defaults to the type's own range).
    fn number(self, value: i64, (lo, hi): (i64, i64)) -> SettingValue {
        let (min, max, step) = match self {
            Self::Range { min, max, step } => (min.max(lo), max.min(hi), step),
            _ => (lo, hi, 1),
        };
        SettingValue::Number {
            value: value.clamp(min, max),
            min,
            max,
            step,
        }
    }
}

/// A config field type that a setting can show and edit.
pub(crate) trait ConfigValue {
    fn to_setting(&self, meta: Meta) -> SettingValue;
    /// Writes `value` back; a value of another kind is ignored.
    fn apply(&mut self, value: &SettingValue);
}

impl ConfigValue for bool {
    fn to_setting(&self, _: Meta) -> SettingValue {
        SettingValue::Toggle(*self)
    }
    fn apply(&mut self, value: &SettingValue) {
        if let SettingValue::Toggle(on) = value {
            *self = *on;
        }
    }
}

/// Integers: a number control, clamped to the type's range.
macro_rules! integer {
    ($($t:ty),*) => {$(
        impl ConfigValue for $t {
            fn to_setting(&self, meta: Meta) -> SettingValue {
                meta.number(as_i64(*self), (as_i64(<$t>::MIN), as_i64(<$t>::MAX)))
            }
            fn apply(&mut self, value: &SettingValue) {
                if let SettingValue::Number { value, .. } = value {
                    *self = from_i64(*value);
                }
            }
        }

        /// "Not set" as the lowest number: `None` ↔ `min` (e.g. `0` = off / default).
        impl ConfigValue for Option<$t> {
            fn to_setting(&self, meta: Meta) -> SettingValue {
                meta.number(self.map_or(0, as_i64), (0, as_i64(<$t>::MAX)))
            }
            fn apply(&mut self, value: &SettingValue) {
                if let SettingValue::Number { value, min, .. } = value {
                    *self = (value > min).then(|| from_i64(*value));
                }
            }
        }
    )*};
}
integer!(u8, u16, u32, u64, i32, i64);

/// Widens to `i64`; `u64` beyond `i64::MAX` saturates.
fn as_i64<T: TryInto<i64>>(n: T) -> i64 {
    n.try_into().unwrap_or(i64::MAX)
}

/// Narrows from `i64`, clamped to the target's range.
fn from_i64<T: TryFrom<i64> + Bounded>(n: i64) -> T {
    T::try_from(n).unwrap_or(if n < 0 { T::MIN } else { T::MAX })
}

/// `MIN` / `MAX` for `from_i64`'s fallback.
trait Bounded: Sized {
    const MIN: Self;
    const MAX: Self;
}
macro_rules! bounded {
    ($($t:ty),*) => {$(
        impl Bounded for $t {
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;
        }
    )*};
}
bounded!(u8, u16, u32, u64, i32, i64);

/// Fractions (`0.0..=1.0`) in percent.
macro_rules! fraction {
    ($($t:ty),*) => {$(
        impl ConfigValue for $t {
            fn to_setting(&self, meta: Meta) -> SettingValue {
                meta.number((*self * 100.).round() as i64, (0, 100))
            }
            fn apply(&mut self, value: &SettingValue) {
                if let SettingValue::Number { value, .. } = value {
                    *self = *value as $t / 100.;
                }
            }
        }
    )*};
}
fraction!(f32, f64);

/// A string: a choice when the descriptor lists the values, free text otherwise.
impl ConfigValue for String {
    fn to_setting(&self, meta: Meta) -> SettingValue {
        match meta {
            Meta::Choices(options) => SettingValue::Choice {
                options,
                selected: options.iter().position(|o| o == self).unwrap_or(0),
            },
            _ => SettingValue::Text(self.clone().into()),
        }
    }
    fn apply(&mut self, value: &SettingValue) {
        match value {
            SettingValue::Choice { options, selected } => {
                if let Some(o) = options.get(*selected) {
                    *self = (*o).to_string();
                }
            }
            SettingValue::Text(text) => *self = text.to_string(),
            _ => {}
        }
    }
}

impl ConfigValue for SharedString {
    fn to_setting(&self, _: Meta) -> SettingValue {
        SettingValue::Text(self.clone())
    }
    fn apply(&mut self, value: &SettingValue) {
        if let SettingValue::Text(text) = value {
            *self = text.clone();
        }
    }
}

/// Optional text: empty means "not set".
impl ConfigValue for Option<String> {
    fn to_setting(&self, _: Meta) -> SettingValue {
        SettingValue::Text(self.clone().unwrap_or_default().into())
    }
    fn apply(&mut self, value: &SettingValue) {
        if let SettingValue::Text(text) = value {
            *self = (!text.is_empty()).then(|| text.to_string());
        }
    }
}
