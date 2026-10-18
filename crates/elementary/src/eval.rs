//! Evaluates expressions to an `f64`.
//!
//! - Trig functions work in degrees.
//! - `a + b%` / `a - b%` mean `a ± a·b/100`; anywhere else `b%` is `b/100`.
//! - `%` (modulo) is Euclidean: `-7 % 3` = 2.
//!
//! `EvalSink` computes values while parsing, so `eval_str` builds no tree.

use smallvec::SmallVec;

use crate::Error;
use crate::func::{Func, constant, factorial};
use crate::parser::{Args, Expr, Op, Sink, parse};

/// Variable bindings, e.g. `&[("x", 2.0)]`. Checked before constants.
pub type Vars<'v> = &'v [(&'v str, f64)];

/// A value plus whether it came straight from a postfix `%`
/// (needed for `200 + 10%` = 220).
#[derive(Debug, Clone, Copy)]
pub struct Val {
    pub v: f64,
    pub pct: bool,
}

impl Val {
    fn plain(v: f64) -> Val {
        Val { v, pct: false }
    }
}

pub struct EvalSink<'v> {
    pub vars: Vars<'v>,
}

impl EvalSink<'_> {
    fn lookup(&self, name: &str) -> Option<f64> {
        self.vars
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .or_else(|| constant(name))
    }
}

impl<'a> Sink<'a> for EvalSink<'_> {
    type Out = Val;

    fn num(&mut self, n: f64) -> Result<Val, Error> {
        Ok(Val::plain(n))
    }

    fn var(&mut self, name: &'a str) -> Result<Val, Error> {
        self.lookup(name)
            .map(Val::plain)
            .ok_or_else(|| Error::UnknownVariable(name.to_string()))
    }

    fn neg(&mut self, e: Val) -> Result<Val, Error> {
        Ok(Val::plain(-e.v))
    }

    fn percent(&mut self, e: Val) -> Result<Val, Error> {
        Ok(Val {
            v: e.v / 100.0,
            pct: true,
        })
    }

    fn factorial(&mut self, e: Val) -> Result<Val, Error> {
        factorial(e.v).map(Val::plain)
    }

    fn binary(&mut self, op: Op, l: Val, r: Val) -> Result<Val, Error> {
        Ok(Val::plain(match (op, r.pct) {
            (Op::Add, true) => l.v + l.v * r.v,
            (Op::Sub, true) => l.v - l.v * r.v,
            _ => apply_op(op, l.v, r.v),
        }))
    }

    fn call(&mut self, name: &'a str, args: Args<Val>) -> Result<Val, Error> {
        if let Some(f) = Func::from_name(name) {
            let a: SmallVec<[f64; 4]> = args.iter().map(|v| v.v).collect();
            return f.apply(&a).map(Val::plain);
        }
        // `pi(2)` / `x(2)` is implicit multiplication
        match (self.lookup(name), args.as_slice()) {
            (Some(c), [x]) => Ok(Val::plain(c * x.v)),
            _ => Err(Error::UnknownFunction(name.to_string())),
        }
    }
}

pub(crate) fn apply_op(op: Op, a: f64, b: f64) -> f64 {
    match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => a / b,
        Op::Mod => a.rem_euclid(b),
        Op::Pow => a.powf(b),
    }
}

fn finite(v: f64) -> Result<f64, Error> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(Error::NotFinite)
    }
}

/// Parse and evaluate in one pass.
pub fn eval_str(src: &str) -> Result<f64, Error> {
    finite(parse(src, &mut EvalSink { vars: &[] })?.v)
}

/// Evaluate a parsed tree with variable bindings.
pub fn eval_with(e: &Expr, vars: Vars) -> Result<f64, Error> {
    finite(e.replay(&mut EvalSink { vars })?.v)
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
