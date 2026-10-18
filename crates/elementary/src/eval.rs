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
    /// The number.
    pub v: f64,
    /// Came straight from a postfix `%`.
    pub pct: bool,
}

impl Val {
    /// A normal value, no percent flag.
    fn plain(v: f64) -> Val {
        Val { v, pct: false }
    }
}

/// Sink that computes values while parsing.
pub struct EvalSink<'v> {
    /// Variable bindings (empty for `eval_str`).
    pub vars: Vars<'v>,
}

impl EvalSink<'_> {
    /// Value of a name: variable first, then constant.
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

    /// `42`
    fn num(&mut self, n: f64) -> Result<Val, Error> {
        Ok(Val::plain(n))
    }

    /// `pi`, `x`
    fn var(&mut self, name: &'a str) -> Result<Val, Error> {
        self.lookup(name)
            .map(Val::plain)
            .ok_or_else(|| Error::UnknownVariable(name.to_string()))
    }

    /// `-a`
    fn neg(&mut self, e: Val) -> Result<Val, Error> {
        Ok(Val::plain(-e.v))
    }

    /// `a%` = `a / 100`, flagged as percent.
    fn percent(&mut self, e: Val) -> Result<Val, Error> {
        Ok(Val {
            v: e.v / 100.0,
            pct: true,
        })
    }

    /// `a!`
    fn factorial(&mut self, e: Val) -> Result<Val, Error> {
        factorial(e.v).map(Val::plain)
    }

    /// `a op b`; `a ± b%` means `a ± a·b`.
    fn binary(&mut self, op: Op, l: Val, r: Val) -> Result<Val, Error> {
        Ok(Val::plain(match (op, r.pct) {
            (Op::Add, true) => l.v + l.v * r.v,
            (Op::Sub, true) => l.v - l.v * r.v,
            _ => apply_op(op, l.v, r.v),
        }))
    }

    /// `sin(a)`, or `pi(a)` as multiplication.
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

/// Plain arithmetic for one binary operator.
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

/// NaN or infinity → `Error::NotFinite`.
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
