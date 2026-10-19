mod error;
pub mod eval;
pub mod solve;
pub mod syntax;

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
    let (lhs, rhs) = syntax::parser::parse_equation(input)?;
    solve::solve(lhs, rhs)
}
