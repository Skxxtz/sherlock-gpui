use crate::Error;
use crate::eval::{
    eval_with,
    func::{Func, constant, factorial},
};
use crate::syntax::ast::{Expr, Op};

/// One stack-machine step: pops its inputs, pushes one result.
#[derive(Debug, Clone, Copy)]
enum Instr {
    /// Push a fixed number.
    Const(f64),
    /// Push the current `x`.
    X,
    /// `-a`
    Neg,
    /// `a%` = `a / 100`
    Pct,
    /// `a!`
    Fact,
    /// `a op b`
    Bin(Op),
    /// `a ^ n` with a constant whole-number `n`: much cheaper than `powf`.
    PowI(i32),
    /// `a + b%` / `a - b%` with `b` raw (not yet divided by 100).
    AddPct,
    SubPct,
    /// Call a function with the top `n` values as arguments.
    Call(Func, u8),
}

/// A compiled expression, ready to run at many `x` values.
pub struct Program {
    /// Instructions, run top to bottom.
    code: Vec<Instr>,
    /// Reused between runs, so running never allocates.
    stack: Vec<f64>,
}

impl Program {
    /// Compile `e` with `var` as the unknown.
    pub fn compile(e: &Expr, var: &str) -> Result<Program, Error> {
        let mut code = Vec::new();
        emit(e, var, &mut code)?;
        Ok(Program {
            code,
            stack: Vec::with_capacity(16),
        })
    }

    /// Number of instructions (shows how much constant folding removed).
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// No instructions at all.
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
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
                    op.apply(s.pop()?, b)
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

/// Append the instructions for `e`; folds parts without `var` to `Const`.
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
