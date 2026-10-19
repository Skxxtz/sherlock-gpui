use smallvec::SmallVec;

use crate::error::Error;

pub type Args<T> = SmallVec<[T; 4]>;

/// Receives parse events bottom-up and builds the output.
pub trait Sink<'a> {
    type Out;
    fn num(&mut self, n: f64) -> Result<Self::Out, Error>;
    fn var(&mut self, name: &'a str) -> Result<Self::Out, Error>;
    fn neg(&mut self, e: Self::Out) -> Result<Self::Out, Error>;
    fn percent(&mut self, e: Self::Out) -> Result<Self::Out, Error>;
    fn factorial(&mut self, e: Self::Out) -> Result<Self::Out, Error>;
    fn binary(&mut self, op: Op, l: Self::Out, r: Self::Out) -> Result<Self::Out, Error>;
    fn call(&mut self, name: &'a str, args: Args<Self::Out>) -> Result<Self::Out, Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl Op {
    /// Plain arithmetic for one binary operator.
    pub(crate) fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div => a / b,
            Op::Mod => a.rem_euclid(b),
            Op::Pow => a.powf(b),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'a> {
    Num(f64),
    Var(&'a str),
    Neg(Box<Expr<'a>>),
    Percent(Box<Expr<'a>>),
    Factorial(Box<Expr<'a>>),
    Binary(Op, Box<Expr<'a>>, Box<Expr<'a>>),
    Call(&'a str, Vec<Expr<'a>>),
}

impl<'a> Expr<'a> {
    /// Does `var` appear anywhere (as a variable or as `var(…)`)?
    pub fn contains(&self, var: &str) -> bool {
        match self {
            Expr::Num(_) => false,
            Expr::Var(n) => *n == var,
            Expr::Neg(e) | Expr::Percent(e) | Expr::Factorial(e) => e.contains(var),
            Expr::Binary(_, l, r) => l.contains(var) || r.contains(var),
            Expr::Call(n, args) => *n == var || args.iter().any(|a| a.contains(var)),
        }
    }

    /// Feed this tree into another sink (e.g. to evaluate it).
    pub fn replay<S: Sink<'a>>(&self, s: &mut S) -> Result<S::Out, Error> {
        match self {
            Expr::Num(n) => s.num(*n),
            Expr::Var(v) => s.var(v),
            Expr::Neg(e) => {
                let e = e.replay(s)?;
                s.neg(e)
            }
            Expr::Percent(e) => {
                let e = e.replay(s)?;
                s.percent(e)
            }
            Expr::Factorial(e) => {
                let e = e.replay(s)?;
                s.factorial(e)
            }
            Expr::Binary(op, l, r) => {
                let l = l.replay(s)?;
                let r = r.replay(s)?;
                s.binary(*op, l, r)
            }
            Expr::Call(n, args) => {
                let args = args
                    .iter()
                    .map(|a| a.replay(s))
                    .collect::<Result<Args<_>, _>>()?;
                s.call(n, args)
            }
        }
    }
}

pub struct TreeSink;

impl<'a> Sink<'a> for TreeSink {
    type Out = Expr<'a>;
    fn num(&mut self, n: f64) -> Result<Expr<'a>, Error> {
        Ok(Expr::Num(n))
    }
    fn var(&mut self, name: &'a str) -> Result<Expr<'a>, Error> {
        Ok(Expr::Var(name))
    }
    fn neg(&mut self, e: Expr<'a>) -> Result<Expr<'a>, Error> {
        Ok(Expr::Neg(Box::new(e)))
    }
    fn percent(&mut self, e: Expr<'a>) -> Result<Expr<'a>, Error> {
        Ok(Expr::Percent(Box::new(e)))
    }
    fn factorial(&mut self, e: Expr<'a>) -> Result<Expr<'a>, Error> {
        Ok(Expr::Factorial(Box::new(e)))
    }
    fn binary(&mut self, op: Op, l: Expr<'a>, r: Expr<'a>) -> Result<Expr<'a>, Error> {
        Ok(Expr::Binary(op, Box::new(l), Box::new(r)))
    }
    fn call(&mut self, name: &'a str, args: Args<Expr<'a>>) -> Result<Expr<'a>, Error> {
        Ok(Expr::Call(name, args.into_vec()))
    }
}
