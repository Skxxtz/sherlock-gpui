//! Compiles a tree with one unknown into flat stack-machine instructions,
//! so the numeric solver can evaluate it thousands of times cheaply.
//! Subtrees without the unknown are folded to constants at compile time.

use crate::Error;
use crate::eval::{apply_op, eval_with};
use crate::func::{Func, constant, factorial};
use crate::parser::{Expr, Op};

#[derive(Debug, Clone, Copy)]
enum Instr {
    Const(f64),
    X,
    Neg,
    Pct,
    Fact,
    Bin(Op),
    /// `a ^ n` with a constant whole-number `n`: much cheaper than `powf`.
    PowI(i32),
    /// `a + b%` / `a - b%` with `b` raw (not yet divided by 100).
    AddPct,
    SubPct,
    Call(Func, u8),
}

pub struct Program {
    code: Vec<Instr>,
    stack: Vec<f64>,
}

impl Program {
    pub fn compile(e: &Expr, var: &str) -> Result<Program, Error> {
        let mut code = Vec::new();
        emit(e, var, &mut code)?;
        Ok(Program {
            code,
            stack: Vec::with_capacity(16),
        })
    }

    /// Evaluate at `x`. `None` if undefined or not finite there.
    pub fn run(&mut self, x: f64) -> Option<f64> {
        let s = &mut self.stack;
        s.clear();
        for ins in &self.code {
            let v = match *ins {
                Instr::Const(c) => c,
                Instr::X => x,
                Instr::Neg => -s.pop()?,
                Instr::Pct => s.pop()? / 100.0,
                Instr::Fact => factorial(s.pop()?).ok()?,
                Instr::PowI(n) => s.pop()?.powi(n),
                Instr::Bin(op) => {
                    let b = s.pop()?;
                    apply_op(op, s.pop()?, b)
                }
                Instr::AddPct | Instr::SubPct => {
                    let p = s.pop()? / 100.0;
                    let a = s.pop()?;
                    if matches!(ins, Instr::AddPct) {
                        a + a * p
                    } else {
                        a - a * p
                    }
                }
                Instr::Call(f, n) => {
                    let start = s.len() - n as usize;
                    let r = f.apply(&s[start..]).ok()?;
                    s.truncate(start);
                    r
                }
            };
            s.push(v);
        }
        s.pop().filter(|v| v.is_finite())
    }
}

fn emit(e: &Expr, var: &str, code: &mut Vec<Instr>) -> Result<(), Error> {
    if !e.contains(var) {
        code.push(Instr::Const(eval_with(e, &[])?));
        return Ok(());
    }
    match e {
        Expr::Num(_) => unreachable!("handled by constant folding"),
        Expr::Var(_) => code.push(Instr::X),
        Expr::Neg(a) => {
            emit(a, var, code)?;
            code.push(Instr::Neg);
        }
        Expr::Percent(a) => {
            emit(a, var, code)?;
            code.push(Instr::Pct);
        }
        Expr::Factorial(a) => {
            emit(a, var, code)?;
            code.push(Instr::Fact);
        }
        Expr::Binary(op @ (Op::Add | Op::Sub), l, r) if matches!(**r, Expr::Percent(_)) => {
            let Expr::Percent(p) = &**r else {
                unreachable!()
            };
            emit(l, var, code)?;
            emit(p, var, code)?;
            code.push(if *op == Op::Add {
                Instr::AddPct
            } else {
                Instr::SubPct
            });
        }
        Expr::Binary(Op::Pow, l, r) if !r.contains(var) => {
            let n = eval_with(r, &[])?;
            emit(l, var, code)?;
            if n.fract() == 0.0 && n.abs() <= 64.0 {
                code.push(Instr::PowI(n as i32));
            } else {
                code.push(Instr::Const(n));
                code.push(Instr::Bin(Op::Pow));
            }
        }
        Expr::Binary(op, l, r) => {
            emit(l, var, code)?;
            emit(r, var, code)?;
            code.push(Instr::Bin(*op));
        }
        Expr::Call(name, args) => {
            if let Some(f) = Func::from_name(name) {
                for a in args {
                    emit(a, var, code)?;
                }
                code.push(Instr::Call(f, args.len() as u8));
                return Ok(());
            }
            // `x(…)` / `pi(…)` = multiplication
            let [arg] = args.as_slice() else {
                return Err(Error::UnknownFunction(name.to_string()));
            };
            if *name == var {
                code.push(Instr::X);
            } else if let Some(c) = constant(name) {
                code.push(Instr::Const(c));
            } else {
                return Err(Error::UnknownFunction(name.to_string()));
            }
            emit(arg, var, code)?;
            code.push(Instr::Bin(Op::Mul));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_tree;

    fn run(src: &str, x: f64) -> Option<f64> {
        Program::compile(&parse_tree(src).unwrap(), "x")
            .unwrap()
            .run(x)
    }

    #[test]
    fn matches_tree_eval() {
        for src in [
            "2x + 1",
            "x² - 5x",
            "x + 19%",
            "x - 25%",
            "sin(x) * 2pi",
            "max(x, 3, 1)",
            "x(x + 2)",
            "(x + 1)!",
            "log(x, 2)",
        ] {
            let tree = parse_tree(src).unwrap();
            for x in [-3.0, 0.5, 2.0, 7.0] {
                let a = run(src, x);
                let b = eval_with(&tree, &[("x", x)]).ok();
                assert_eq!(a, b, "{src} at x = {x}");
            }
        }
    }

    #[test]
    fn folds_constants() {
        let p = Program::compile(&parse_tree("x + sqrt(16) * 2").unwrap(), "x").unwrap();
        assert_eq!(p.code.len(), 3); // X, Const(8), Add
    }

    #[test]
    fn undefined_is_none() {
        assert_eq!(run("1 / x", 0.0), None);
        assert_eq!(run("tan(x)", 90.0), None);
        assert_eq!(run("sqrt(x)", -1.0), None);
    }
}
