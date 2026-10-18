//! Built-in functions and constants. Names are resolved once to `Func`.

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Sqrt,
    Cbrt,
    Abs,
    Exp,
    Ln,
    Log2,
    Log,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Floor,
    Ceil,
    Round,
    Min,
    Max,
}

impl Func {
    pub fn from_name(name: &str) -> Option<Func> {
        use Func::*;
        Some(match name {
            "sqrt" => Sqrt,
            "cbrt" => Cbrt,
            "abs" => Abs,
            "exp" => Exp,
            "ln" => Ln,
            "log2" => Log2,
            "log" => Log,
            "sin" => Sin,
            "cos" => Cos,
            "tan" => Tan,
            "asin" => Asin,
            "acos" => Acos,
            "atan" => Atan,
            "sinh" => Sinh,
            "cosh" => Cosh,
            "tanh" => Tanh,
            "floor" => Floor,
            "ceil" => Ceil,
            "round" => Round,
            "min" => Min,
            "max" => Max,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        use Func::*;
        match self {
            Sqrt => "sqrt",
            Cbrt => "cbrt",
            Abs => "abs",
            Exp => "exp",
            Ln => "ln",
            Log2 => "log2",
            Log => "log",
            Sin => "sin",
            Cos => "cos",
            Tan => "tan",
            Asin => "asin",
            Acos => "acos",
            Atan => "atan",
            Sinh => "sinh",
            Cosh => "cosh",
            Tanh => "tanh",
            Floor => "floor",
            Ceil => "ceil",
            Round => "round",
            Min => "min",
            Max => "max",
        }
    }

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

pub fn factorial(n: f64) -> Result<f64, Error> {
    if n < 0.0 || n.fract() != 0.0 {
        return Err(Error::Domain("factorial needs a non-negative integer"));
    }
    if n > 170.0 {
        return Err(Error::NotFinite); // 171! overflows f64
    }
    Ok((2..=n as u64).fold(1.0, |acc, k| acc * k as f64))
}
