//! `elementary` – a small math expression evaluator (meval replacement).
//!
//! ```
//! assert_eq!(elementary::eval_str("200 + 10%").unwrap(), 220.0);
//! assert_eq!(elementary::eval_str("2sin(90) + 3!").unwrap(), 8.0);
//!
//! let s = elementary::solve_str("x² - 5x + 6 = 0").unwrap();
//! assert_eq!(s.roots, vec![elementary::Root::Real(2.0), elementary::Root::Real(3.0)]);
//! ```

pub mod compile;
mod error;
pub mod eval;
pub mod func;
pub mod lexer;
pub mod parser;
pub mod solve;

pub use error::Error;
pub use solve::{Root, Solution};

/// Parse and evaluate an expression string.
pub fn eval_str<S: AsRef<str>>(expr: S) -> Result<f64, Error> {
    let expr = expr.as_ref().trim();
    if expr.is_empty() {
        return Err(Error::Empty);
    }
    eval::eval_str(expr)
}

/// Solve an equation with one unknown, e.g. `x² - 5x + 6 = 0`.
pub fn solve_str<S: AsRef<str>>(equation: S) -> Result<Solution, Error> {
    let input = equation.as_ref().trim();
    if input.is_empty() {
        return Err(Error::Empty);
    }
    let (lhs, rhs) = parser::parse_equation(input)?;
    solve::solve(lhs, rhs)
}
