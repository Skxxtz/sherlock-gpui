//! Evaluates an expression tree to an `f64`.
//!
//! - Trig functions work in degrees.
//! - `a + b%` / `a - b%` mean `a ± a·b/100`; anywhere else `b%` is `b/100`.
//! - `%` (modulo) is Euclidean: `-7 % 3` = 2.

use crate::Error;
use crate::parser::{Expr, Op};

pub fn eval(e: &Expr) -> Result<f64, Error> {
    let v = eval_inner(e)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(Error::NotFinite)
    }
}

fn eval_inner(e: &Expr) -> Result<f64, Error> {
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Var(name) => constant(name).ok_or_else(|| Error::UnknownVariable(name.clone()))?,
        Expr::Neg(e) => -eval_inner(e)?,
        Expr::Percent(e) => eval_inner(e)? / 100.0,
        Expr::Factorial(e) => factorial(eval_inner(e)?)?,
        Expr::Binary(op, l, r) => {
            let a = eval_inner(l)?;
            // calculator-style percent: `200 + 10%` = 220
            if let (Op::Add | Op::Sub, Expr::Percent(p)) = (op, r.as_ref()) {
                let delta = a * eval_inner(p)? / 100.0;
                return Ok(if *op == Op::Add { a + delta } else { a - delta });
            }
            let b = eval_inner(r)?;
            match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Mod => a.rem_euclid(b),
                Op::Pow => a.powf(b),
            }
        }
        Expr::Call(name, args) => {
            let args = args.iter().map(eval_inner).collect::<Result<Vec<_>, _>>()?;
            call(name, &args)?
        }
    })
}

fn constant(name: &str) -> Option<f64> {
    use std::f64::consts;
    Some(match name {
        "pi" | "π" => consts::PI,
        "tau" | "τ" => consts::TAU,
        "e" => consts::E,
        "phi" | "φ" => 1.618_033_988_749_895,
        _ => return None,
    })
}

fn call(name: &str, a: &[f64]) -> Result<f64, Error> {
    let arity = |expected: &'static str, ok: bool| {
        if ok {
            Ok(())
        } else {
            Err(Error::WrongArgCount {
                name: name.to_string(),
                expected,
                got: a.len(),
            })
        }
    };
    let one = |f: fn(f64) -> f64| -> Result<f64, Error> {
        arity("1", a.len() == 1)?;
        Ok(f(a[0]))
    };

    match name {
        "sqrt" => one(f64::sqrt),
        "cbrt" => one(f64::cbrt),
        "abs" => one(f64::abs),
        "exp" => one(f64::exp),
        "ln" => one(f64::ln),
        "log2" => one(f64::log2),
        "log" => {
            arity("1 or 2", matches!(a.len(), 1 | 2))?;
            Ok(if a.len() == 1 {
                a[0].log10()
            } else {
                a[0].log(a[1])
            })
        }
        "sin" => one(sin_deg),
        "cos" => one(|x| sin_deg(x + 90.0)),
        "tan" => {
            arity("1", a.len() == 1)?;
            let c = sin_deg(a[0] + 90.0);
            if c == 0.0 {
                return Err(Error::Domain("tan is undefined here"));
            }
            Ok(sin_deg(a[0]) / c)
        }
        "asin" => one(|x| x.asin().to_degrees()),
        "acos" => one(|x| x.acos().to_degrees()),
        "atan" => one(|x| x.atan().to_degrees()),
        "sinh" => one(f64::sinh),
        "cosh" => one(f64::cosh),
        "tanh" => one(f64::tanh),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "round" => one(f64::round),
        "min" | "max" => {
            arity("at least 1", !a.is_empty())?;
            let pick = if name == "min" { f64::min } else { f64::max };
            Ok(a.iter().copied().reduce(pick).unwrap())
        }
        // implicit multiplication with a constant: `pi(2)` = `pi * 2`
        _ => match (constant(name), a) {
            (Some(c), [x]) => Ok(c * x),
            _ => Err(Error::UnknownFunction(name.to_string())),
        },
    }
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

fn factorial(n: f64) -> Result<f64, Error> {
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
    use crate::{Error, eval_str};

    fn ev(s: &str) -> f64 {
        eval_str(s).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn arithmetic() {
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("2^10"), 1024.0);
        assert_eq!(ev("-3²"), -9.0);
        assert_eq!(ev("2^-1"), 0.5);
        assert_eq!(ev("10 % 3"), 1.0);
        assert_eq!(ev("-7 % 3"), 2.0);
        assert!(close(ev("2pi"), std::f64::consts::TAU));
        assert_eq!(ev("2(3+4)"), 14.0);
    }

    #[test]
    fn percent() {
        assert_eq!(ev("50%"), 0.5);
        assert_eq!(ev("200 + 10%"), 220.0);
        assert_eq!(ev("80 - 25%"), 60.0);
        assert_eq!(ev("200 * 15%"), 30.0);
        assert_eq!(ev("50% * 8"), 4.0);
        assert!(close(ev("100 + 19%"), 119.0));
    }

    #[test]
    fn trig_degrees() {
        assert_eq!(ev("sin(90)"), 1.0);
        assert_eq!(ev("sin(180)"), 0.0);
        assert_eq!(ev("cos(180)"), -1.0);
        assert_eq!(ev("cos(90)"), 0.0);
        assert!(close(ev("cos(60)"), 0.5));
        assert!(close(ev("tan(45)"), 1.0));
        assert!(close(ev("asin(1)"), 90.0));
        assert_eq!(
            eval_str("tan(90)"),
            Err(Error::Domain("tan is undefined here"))
        );
    }

    #[test]
    fn functions() {
        assert_eq!(ev("sqrt(16)"), 4.0);
        assert_eq!(ev("log(1000)"), 3.0);
        assert_eq!(ev("log(8, 2)"), 3.0);
        assert!(close(ev("ln(e)"), 1.0));
        assert_eq!(ev("max(1, 5, 3)"), 5.0);
        assert_eq!(ev("min(4, -2)"), -2.0);
        assert_eq!(ev("round(2.5)"), 3.0);
        assert!(close(ev("pi(2)"), std::f64::consts::TAU));
    }

    #[test]
    fn factorial() {
        assert_eq!(ev("0!"), 1.0);
        assert_eq!(ev("5!"), 120.0);
        assert_eq!(ev("3!^2"), 36.0);
        assert_eq!(ev("-3!"), -6.0);
        assert!(matches!(eval_str("2.5!"), Err(Error::Domain(_))));
        assert!(matches!(eval_str("(-1)!"), Err(Error::Domain(_))));
        assert_eq!(eval_str("171!"), Err(Error::NotFinite));
    }

    #[test]
    fn errors() {
        assert_eq!(eval_str("1/0"), Err(Error::NotFinite));
        assert_eq!(eval_str("sqrt(-1)"), Err(Error::NotFinite));
        assert_eq!(
            eval_str("foo + 1"),
            Err(Error::UnknownVariable("foo".into()))
        );
        assert_eq!(
            eval_str("foo(1)"),
            Err(Error::UnknownFunction("foo".into()))
        );
        assert!(matches!(
            eval_str("sin(1, 2)"),
            Err(Error::WrongArgCount { .. })
        ));
    }
}
