/// A config enum: a fixed set of strings, shown as a choice in the settings.
pub trait ConfigEnum: Copy + 'static {
    /// The values as written in the file, in variant order.
    const OPTIONS: &'static [&'static str];
    fn index(self) -> usize;
    fn from_index(i: usize) -> Option<Self>;
}

/// Defines a config enum. Each variant has the string it's written as, then other spellings
/// it's read from (case-insensitive). An unknown value falls back to the default, with a warning.
macro_rules! config_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $var:ident = $canon:literal $(| $alias:literal)* ),+ $(,)?
        }
        default = $default:ident;
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $var ),+
        }

        impl Default for $name {
            fn default() -> Self {
                Self::$default
            }
        }

        impl $name {
            const ALL: &'static [Self] = &[$(Self::$var),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$var => $canon),+
                }
            }

            pub fn parse(s: &str) -> Option<Self> {
                let s = s.trim();
                $(
                    if s.eq_ignore_ascii_case($canon) $(|| s.eq_ignore_ascii_case($alias))* {
                        return Some(Self::$var);
                    }
                )+
                None
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl $crate::config::types::ConfigEnum for $name {
            const OPTIONS: &'static [&'static str] = &[$($canon),+];
            fn index(self) -> usize {
                Self::ALL.iter().position(|&v| v == self).unwrap_or(0)
            }
            fn from_index(i: usize) -> Option<Self> {
                Self::ALL.get(i).copied()
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Ok(Self::parse(&s).unwrap_or_else(|| {
                    let default = Self::default();
                    $crate::config::types::warn(format!(
                        "unknown {} `{}` (expected one of: {}), using `{}`",
                        stringify!($name),
                        s,
                        <Self as $crate::config::types::ConfigEnum>::OPTIONS.join(", "),
                        default,
                    ));
                    default
                }))
            }
        }
    };
}

config_enum! {
    /// A screen edge.
    pub enum Edge {
        Top = "top",
        Bottom = "bottom",
        Left = "left",
        Right = "right",
    }
    default = Top;
}

config_enum! {
    /// Where the window sits on the screen.
    pub enum Anchor {
        Center = "center" | "" | "centre",
        Top = "top",
        Bottom = "bottom",
        Left = "left",
        Right = "right",
        TopLeft = "top-left" | "top_left",
        TopRight = "top-right" | "top_right",
        BottomLeft = "bottom-left" | "bottom_left",
        BottomRight = "bottom-right" | "bottom_right",
    }
    default = Center;
}

config_enum! {
    pub enum TemperatureUnit {
        Celsius = "C" | "celsius" | "°c",
        Fahrenheit = "F" | "fahrenheit" | "°f",
    }
    default = Celsius;
}

config_enum! {
    pub enum LengthUnit {
        Meters = "meters" | "meter" | "metres" | "metre" | "m",
        Kilometers = "kilometers" | "kilometer" | "kilometres" | "km",
        Centimeters = "centimeters" | "centimeter" | "cm",
        Millimeters = "millimeters" | "millimeter" | "mm",
        Inches = "inches" | "inch" | "in",
        Feet = "feet" | "foot" | "ft",
        Yards = "yards" | "yard" | "yd",
        Miles = "miles" | "mile" | "mi",
    }
    default = Meters;
}

impl LengthUnit {
    pub fn is_imperial(self) -> bool {
        matches!(self, Self::Inches | Self::Feet | Self::Yards | Self::Miles)
    }
}

config_enum! {
    pub enum WeightUnit {
        Kilograms = "kg" | "kilogram" | "kilograms",
        Grams = "g" | "gram" | "grams",
        Pounds = "lb" | "lbs" | "pound" | "pounds",
        Ounces = "oz" | "ounce" | "ounces",
    }
    default = Kilograms;
}

config_enum! {
    pub enum VolumeUnit {
        Liters = "l" | "liter" | "liters" | "litre" | "litres",
        Milliliters = "ml" | "milliliter" | "milliliters",
        Gallons = "gal" | "gallon" | "gallons",
        FluidOunces = "floz" | "fl oz" | "fl_oz",
    }
    default = Liters;
}
