//! Built-in functions and constants. Names are resolved once to `Func`.

use crate::Error;

/// Declares the function enum plus `from_name` and `name`, both generated
/// from the `#[name = "…"]` on each variant.
macro_rules! functions {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident {
            $( #[name = $name:literal] $Variant:ident ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $Enum {
            $( $Variant ),*
        }

        impl $Enum {
            /// `"sin"` → `Func::Sin`; `None` if unknown.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $( $name => Some(Self::$Variant), )*
                    _ => None,
                }
            }

            /// `Func::Sin` → `"sin"` (for error messages).
            pub fn name(self) -> &'static str {
                match self {
                    $( Self::$Variant => $name, )*
                }
            }
        }
    };
}

functions! {
    /// A built-in function. Trig works in degrees.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Func {
        #[name = "sqrt"]
        Sqrt,
        #[name = "cbrt"]
        Cbrt,
        #[name = "abs"]
        Abs,
        #[name = "exp"]
        Exp,
        #[name = "ln"]
        Ln,
        #[name = "log2"]
        Log2,
        #[name = "log"]
        Log,
        #[name = "sin"]
        Sin,
        #[name = "cos"]
        Cos,
        #[name = "tan"]
        Tan,
        #[name = "asin"]
        Asin,
        #[name = "acos"]
        Acos,
        #[name = "atan"]
        Atan,
        #[name = "sinh"]
        Sinh,
        #[name = "cosh"]
        Cosh,
        #[name = "tanh"]
        Tanh,
        #[name = "floor"]
        Floor,
        #[name = "ceil"]
        Ceil,
        #[name = "round"]
        Round,
        #[name = "min"]
        Min,
        #[name = "max"]
        Max,
    }
}

impl Func {
    /// Run with arguments `a`; checks the argument count.
    pub fn apply(self, a: &[f64]) -> Result<f64, Error> {
        use Func::*;
        let arity = |expected: &'static str, ok: bool| {
            if ok {
                Ok(())
            } else {
                Err(Error::WrongArgCount {
                    name: self.name().to_string(),
                    expected,
                    got: a.len(),
                })
            }
        };
        if !matches!(self, Log | Min | Max) {
            arity("1", a.len() == 1)?;
        }
        Ok(match self {
            Sqrt => a[0].sqrt(),
            Cbrt => a[0].cbrt(),
            Abs => a[0].abs(),
            Exp => a[0].exp(),
            Ln => a[0].ln(),
            Log2 => a[0].log2(),
            Log => {
                arity("1 or 2", matches!(a.len(), 1 | 2))?;
                if a.len() == 1 {
                    a[0].log10()
                } else {
                    a[0].log(a[1])
                }
            }
            Sin => sin_deg(a[0]),
            Cos => sin_deg(a[0] + 90.0),
            Tan => {
                let c = sin_deg(a[0] + 90.0);
                if c == 0.0 {
                    return Err(Error::Domain("tan is undefined here"));
                }
                sin_deg(a[0]) / c
            }
            Asin => a[0].asin().to_degrees(),
            Acos => a[0].acos().to_degrees(),
            Atan => a[0].atan().to_degrees(),
            Sinh => a[0].sinh(),
            Cosh => a[0].cosh(),
            Tanh => a[0].tanh(),
            Floor => a[0].floor(),
            Ceil => a[0].ceil(),
            Round => a[0].round(),
            Min | Max => {
                arity("at least 1", !a.is_empty())?;
                let pick = if self == Min { f64::min } else { f64::max };
                a.iter().copied().reduce(pick).unwrap()
            }
        })
    }
}

/// Value of a named constant (`pi`, `e`, …); `None` if unknown.
pub fn constant(name: &str) -> Option<f64> {
    use std::f64::consts;
    Some(match name {
        "pi" | "π" => consts::PI,
        "tau" | "τ" => consts::TAU,
        "e" => consts::E,
        "phi" | "φ" => 1.618_033_988_749_895,
        _ => return None,
    })
}

/// Sine in degrees, exact at multiples of 90° (so `sin(180)` is 0, not 1.2e-16).
fn sin_deg(x: f64) -> f64 {
    let r = x.rem_euclid(360.0);
    match r {
        0.0 | 180.0 => 0.0,
        90.0 => 1.0,
        270.0 => -1.0,
        _ => r.to_radians().sin(),
    }
}

/// `n!` for whole numbers 0 to 170.
pub fn factorial(n: f64) -> Result<f64, Error> {
    if n < 0.0 || n.fract() != 0.0 {
        return Err(Error::Domain("factorial needs a non-negative integer"));
    }
    if n > 170.0 {
        return Err(Error::NotFinite); // 171! overflows f64
    }
    Ok((2..=n as u64).fold(1.0, |acc, k| acc * k as f64))
}

#[cfg(test)]
mod tests {
    use super::Func;

    #[test]
    fn names_round_trip() {
        for name in ["sqrt", "log2", "sin", "max"] {
            assert_eq!(Func::from_name(name).unwrap().name(), name);
        }
        assert_eq!(Func::from_name("foo"), None);
    }
}
