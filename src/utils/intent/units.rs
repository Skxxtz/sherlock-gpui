use crate::launcher::calc_launcher::CURRENCIES;

macro_rules! define_units {
    ($(
        $category:ident, $cap_const:ident {
            cap: $cap_val:expr,
            id: $id:literal,
            $($variant:ident: [$($alias:literal),*] => $factor:expr, $canonical_symbol:literal),* $(,)?
        }
    )*) => {
        #[derive(PartialEq, Eq, Hash, Clone, Copy)]
        #[allow(dead_code)]
        pub enum UnitCategory { $($category),* }
        #[allow(dead_code)]
        impl UnitCategory {
            pub fn capability_mask(&self) -> u32 {
                match self {
                    $( UnitCategory::$category => Capabilities::$cap_const, )*
                }
            }
        }

        #[derive(Clone, Copy)]
        pub struct Capabilities(pub u32);
        #[allow(dead_code)]
        impl Capabilities {
            pub const NONE: u32 = 0;
            $( pub const $cap_const: u32 = $cap_val; )*
            pub const EVERYTHING: u32 = u32::MAX;

            #[inline]
            pub fn allows(&self, cap: u32) -> bool {
                (self.0 & cap) != 0
            }
        }

        #[cfg(feature = "docs")]
        pub mod docs {
            pub struct CapabilityDoc {
                pub name: &'static str,
                pub identifier: &'static str,
                pub units: &'static [UnitDoc],
            }

            pub struct UnitDoc {
                pub name: &'static str,
                pub aliases: &'static [&'static str],
                pub symbol: &'static str,
            }

            pub static CAPABILITY_DOCS: &[CapabilityDoc] = &[
                $(
                    CapabilityDoc {
                        name: stringify!($category),
                        identifier: $id,
                        units: &[
                            $(
                                UnitDoc {
                                    name: stringify!($variant),
                                    aliases: &[$($alias),*],
                                    symbol: $canonical_symbol,
                                },
                            )*
                        ],
                    },
                )*
            ];
        }

        impl std::fmt::Debug for Capabilities {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut active: Vec<&'static str> = Vec::new();
                $( if self.allows(Self::$cap_const) { active.push(stringify!($cap_const)); } )*
                write!(f, "Capabilities({})", active.join(" | "))
            }
        }

        impl std::ops::BitOr for Capabilities {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitOrAssign<u32> for Capabilities {
            fn bitor_assign(&mut self, rhs: u32) {
                self.0 |= rhs;
            }
        }

        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub enum Unit {
            $( $( $variant, )* )*
        }

        impl Unit {
            pub fn category(&self) -> UnitCategory {
                match self {
                    $( $(Unit::$variant => UnitCategory::$category,)* )*
                }
            }

            pub fn symbol(&self) -> &'static str {
                match self {
                    $( $(Unit::$variant => $canonical_symbol,)* )*
                }
            }

            // The raw factor (for static units)
            fn raw_factor(&self) -> f64 {
                match self {
                    $( $(Unit::$variant => $factor as f64,)* )*
                }
            }

            pub(super) fn parse_in_category(s: &str, cat: UnitCategory) -> Option<Self> {
                let s = s.trim().to_lowercase();
                if s.is_empty() { return None; } // Guard against empty strings

                match cat {
                    $(
                        UnitCategory::$category => {
                            // 1. Exact Match Path
                            $(
                                if [$($alias),*].contains(&s.as_str()) {
                                    return Some(Unit::$variant);
                                }
                            )*
                                if s.len() >= 2 {
                                    $(
                                        for alias in [$($alias),*] {
                                            if alias.starts_with(&s) {
                                                return Some(Unit::$variant);
                                            }
                                        }
                                    )*
                                }
                            None
                        },
                    )*
                }
            }

            pub fn parse_with_capabilities(s: &str, caps: &Capabilities) -> Option<Self> {
                let s = s.trim();
                if s.is_empty() { return None; }
                let s_lower = s.to_lowercase();
                let s_ptr = s_lower.as_str();

                $(
                    if caps.allows(Capabilities::$cap_const) {
                        $(
                            if [$($alias),*].contains(&s_ptr) {
                                return Some(Unit::$variant);
                            }
                        )*
                    }
                )*

                    if s_lower.len() >= 3 {
                        $(
                            if caps.allows(Capabilities::$cap_const) {
                                $(
                                    for alias in [$($alias),*] {
                                        if alias.len() > s_lower.len() && alias.starts_with(&s_lower) {
                                            return Some(Unit::$variant);
                                        }
                                    }
                                )*
                            }
                        )*
                    }

                None
            }
        }
    };
}

impl Capabilities {
    pub fn from_strings(strs: &[String]) -> Self {
        let mut mask = Self::NONE;
        for s in strs {
            mask |= match s.as_str() {
                "calc.currencies" => Self::CURRENCY,
                "calc.math" => Self::MATH,
                "colors" => Self::COLORS,

                // all units
                "calc.units" => {
                    Self::LENGTH
                        | Self::VOLUME
                        | Self::WEIGHT
                        | Self::TEMPERATURE
                        | Self::PRESSURE
                        | Self::DIGITAL
                        | Self::TIME
                        | Self::AREA
                        | Self::SPEED
                }

                // individual units
                "calc.length" => Self::LENGTH,
                "calc.volume" => Self::VOLUME,
                "calc.weight" => Self::WEIGHT,
                "calc.temperature" => Self::TEMPERATURE,
                "calc.pressure" => Self::PRESSURE,
                "calc.digital" => Self::DIGITAL,
                "calc.time" => Self::TIME,
                "calc.area" => Self::AREA,
                "calc.speed" => Self::SPEED,

                _ => Self::NONE,
            }
        }

        Self(mask)
    }
}

impl Unit {
    pub fn factor(&self) -> f64 {
        // use dynamic factors for currencies
        if self.category() == UnitCategory::Currency
            && let Some(Some(rates)) = CURRENCIES.get()
        {
            let rate = match self {
                Unit::Usd => rates.usd,
                Unit::Eur => rates.eur,
                Unit::Jpy => rates.jpy,
                Unit::Gbp => rates.gbp,
                Unit::Aud => rates.aud,
                Unit::Cad => rates.cad,
                Unit::Chf => rates.chf,
                Unit::Cny => rates.cny,
                Unit::Nzd => rates.nzd,
                Unit::Sek => rates.sek,
                Unit::Nok => rates.nok,
                Unit::Mxn => rates.mxn,
                Unit::Sgd => rates.sgd,
                Unit::Hkd => rates.hkd,
                Unit::Krw => rates.krw,
                Unit::Pln => rates.pln,
                Unit::Pen => rates.pen,
                _ => 1.0,
            };
            return rate as f64;
        }

        // use hardcoded factor
        self.raw_factor()
    }
}

define_units! {
    Math, MATH {
        cap: 1 << 0,
        id: "calc.math",
    }
    Colors, COLORS {
        cap: 1 << 1,
        id: "colors",
    }
    Currency, CURRENCY {
        cap: 1 << 2,
        id: "calc.currencies",
        Usd: ["usd", "dollar", "dollars", "bucks", "$"] => 1.0, "$",
        Eur: ["eur", "euro", "euros", "€"] => 1.0, "€",
        Jpy: ["jpy", "yen", "japanese yen", "¥"] => 1.0, "¥",
        Gbp: ["gbp", "pound", "pounds", "sterling", "£"] => 1.0, "£",
        Aud: ["aud", "australian dollar", "aussie", "a$"] => 1.0, "A$",
        Cad: ["cad", "canadian dollar", "loonie", "c$"] => 1.0, "C$",
        Chf: ["chf", "swiss franc", "franc"] => 1.0, "CHF",
        Cny: ["cny", "chinese yuan", "renminbi", "yuan"] => 1.0, "¥",
        Nzd: ["nzd", "new zealand dollar", "kiwi", "nz$"] => 1.0, "NZ$",
        Sek: ["sek", "swedish krona", "krona", "kr"] => 1.0, "kr",
        Nok: ["nok", "norwegian krone", "krone"] => 1.0, "kr",
        Mxn: ["mxn", "mexican peso", "peso", "mex$"] => 1.0, "Mex$",
        Sgd: ["sgd", "singapore dollar", "s$"] => 1.0, "S$",
        Hkd: ["hkd", "hong kong dollar", "hk$"] => 1.0, "HK$",
        Krw: ["krw", "south korean won", "won", "₩"] => 1.0, "₩",
        Pln: ["pln", "polish", "złoty", "zł"] => 1.0, "zł",
        Pen: ["pen", "peruvian", "sole", "soles"] => 1.0, "S/",
    }
    Length, LENGTH {
        cap: 1 << 3,
        id: "calc.length",
        Millimeter: ["mm", "millimeter", "millimeters"] => 0.001, "mm",
        Centimeter: ["cm", "centimeter", "centimeters"] => 0.01, "cm",
        Meter: ["m", "meter", "meters"] => 1.0, "m",
        Kilometer: ["km", "kilometer", "kilometers", "kilos"] => 1000.0, "km",
        Inch: ["in", "inch", "inches", "\""] => 0.0254, "in",
        Feet: ["ft", "feet", "foot", "'"] => 0.3048, "ft",
        Yard: ["yd", "yard", "yards"] => 0.9144, "yd",
        Mile: ["mi", "mile", "miles"] => 1609.34, "mi",
        NauticalMile: ["nm", "nautical mile"] => 1852.0, "nmi",
    }
    Volume, VOLUME {
        cap: 1 << 4,
        id: "calc.volume",
        Milliliter: ["ml", "milliliter", "milliliters", "cc"] => 0.001, "ml",
        Centiliter: ["cl", "centiliter"] => 0.01, "cl",
        Liter: ["l", "liter", "liters"] => 1.0, "l",
        Kiloliter: ["kl", "kiloliter"] => 1000.0, "kl",
        CubicMeter: ["m3", "cubic meter", "cubic meters"] => 1000.0, "m³",
        // US Liquid
        Teaspoon: ["tsp", "teaspoon"] => 0.00492892, "tsp",
        Tablespoon: ["tbsp", "tablespoon"] => 0.0147868, "tbsp",
        FluidOunce: ["fl oz", "fluid ounce", "fluid ounces"] => 0.0295735, "fl oz",
        Cup: ["cup", "cups"] => 0.236588, "cup",
        Pint: ["pt", "pint", "pints"] => 0.473176, "pt",
        Quart: ["qt", "quart", "quarts"] => 0.946353, "qt",
        Gallon: ["gal", "gallon", "gallons"] => 3.78541, "gal",
        // Imperial
        ImperialGallon: ["imp gal"] => 4.54609, "imp gal",
    }
    Weight, WEIGHT {
        cap: 1 << 5,
        id: "calc.weight",
        Milligram: ["mg", "milligram", "milligrams"] => 0.000001, "mg",
        Gram: ["g", "gram", "grams"] => 0.001, "g",
        Kilogram: ["kg", "kilogram", "kilograms", "kilo", "kilos"] => 1.0, "kg",
        MetricTon: ["t", "tonne", "metric ton", "metric tons"] => 1000.0, "t",
        // Imperial/US
        Ounce: ["oz", "ounce", "ounces"] => 0.0283495, "oz",
        Pound: ["lb", "lbs", "pound", "pounds"] => 0.453592, "lb",
        Stone: ["st", "stone", "stones"] => 6.35029, "st",
        ShortTon: ["ton", "tons", "us ton"] => 907.185, "ton",
        LongTon: ["imperial ton", "uk ton"] => 1016.05, "ton",
        // Precious Metals
        TroyOunce: ["ozt", "troy ounce", "troy ounces"] => 0.0311035, "ozt",
    }
    Temperature, TEMPERATURE {
        cap: 1 << 6,
        id: "calc.temperature",
        Celsius: ["c", "celsius", "°c", "°"] => 1.0, "°C",
        Fahrenheit: ["f", "fahrenheit", "°f"] => 1.0, "°F",
    }
    Pressure, PRESSURE {
        cap: 1 << 7,
        id: "calc.pressure",
        Pascal: ["pa", "pascal", "pascals"] => 0.00001, "Pa",
        Kilopascal: ["kpa", "kilopascal"] => 0.01, "kPa",
        Bar: ["bar", "bars"] => 1.0, "bar",
        Atmosphere: ["atm", "atmosphere", "atmospheres"] => 1.01325, "atm",
        Psi: ["psi", "pounds per square inch"] => 0.06894757, "psi",
        Torr: ["torr", "mmhg"] => 0.00133322, "mmHg",
    }
    Digital, DIGITAL {
        cap: 1 << 8,
        id: "calc.digital",
        Bit: ["bit", "bits", "b"] => 0.125, "bit",
        Kilobit: ["kb", "kilobit"] => 128.0, "kb",
        Megabit: ["mb", "megabit"] => 131072.0, "Mb",
        Gigabit: ["gb", "gigabit"] => 134217728.0, "Gb",
        Byte: ["byte", "bytes", "B"] => 1.0, "B",
        Kilobyte: ["kb", "kilobyte", "KB"] => 1024.0, "KB",
        Megabyte: ["mb", "megabyte", "MB"] => 1048576.0, "MB",
        Gigabyte: ["gb", "gigabyte", "GB"] => 1073741824.0, "GB",
        Terabyte: ["tb", "terabyte", "TB"] => 1099511627776.0, "TB",
        Petabyte: ["pb", "petabyte", "PB"] => 1125899906842624.0, "PB",
    }
    Time, TIME {
        cap: 1 << 9,
        id: "calc.time",
        Milliseconds: ["ms", "millisecond", "milliseconds"] => 0.001, "ms",
        Seconds: ["s", "sec", "second", "seconds"] => 1.0, "s",
        Minutes: ["m", "min", "minute", "minutes"] => 60.0, "min",
        Hours: ["h", "hr", "hour", "hours"] => 3600.0, "h",
        Days: ["d", "day", "days"] => 86400.0, "d",
        Weeks: ["wk", "week", "weeks"] => 604800.0, "wk",
        Months: ["mo", "month", "months"] => 2629746.0, "mo",
        Years: ["yr", "year", "years"] => 31556952.0, "yr",
    }
    Area, AREA {
        cap: 1 << 10,
        id: "calc.area",
        SquareMeter: ["m2", "sq m", "sq meter"] => 1.0, "m²",
        SquareKilometer: ["km2", "sq km"] => 1000000.0, "km²",
        SquareFoot: ["ft2", "sq ft", "sq feet"] => 0.092903, "ft²",
        SquareInch: ["in2", "sq in"] => 0.00064516, "in²",
        Acre: ["acre", "acres"] => 4046.86, "ac",
        Hectare: ["ha", "hectare"] => 10000.0, "ha",
    }
    Speed, SPEED {
        cap: 1 << 11,
        id: "calc.speed",
        MetersPerSecond: ["ms", "m/s", "meters per second"] => 1.0, "m/s",
        KilometersPerHour: ["kmh", "km/h", "kph"] => 0.277778, "km/h",
        MilesPerHour: ["mph", "mile per hour", "miles per hour"] => 0.44704, "mph",
        Knot: ["kn", "knot", "knots"] => 0.514444, "kn",
    }
}
