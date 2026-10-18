//! `elementary` – a small math expression evaluator (meval replacement).
//!
//! ```
//! assert_eq!(elementary::eval_str("200 + 10%").unwrap(), 220.0);
//! assert_eq!(elementary::eval_str("2sin(90) + 3!").unwrap(), 8.0);
//!
//! let s = elementary::solve_str("x² - 5x + 6 = 0").unwrap();
//! assert_eq!(s.roots, vec![elementary::Root::Real(2.0), elementary::Root::Real(3.0)]);
//! ```

mod error;
pub mod eval;
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
    let tokens = lexer::tokenize(expr)?;
    let tree = parser::parse(&tokens)?;
    eval::eval(&tree)
}

/// Solve an equation with one unknown, e.g. `x² - 5x + 6 = 0`.
pub fn solve_str<S: AsRef<str>>(equation: S) -> Result<Solution, Error> {
    let input = equation.as_ref().trim();
    if input.is_empty() {
        return Err(Error::Empty);
    }
    let tokens = lexer::tokenize(input)?;
    let (lhs, rhs) = parser::parse_equation(&tokens)?;
    solve::solve(&lhs, &rhs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        eval_str(s).unwrap_or_else(|e| panic!("{s:?} failed: {e}"))
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn empty_input() {
        assert_eq!(eval_str("  "), Err(Error::Empty));
    }

    #[test]
    fn basic_math() {
        assert_eq!(ev("1 + 1"), 2.0);
        assert_eq!(ev("10 - 4 / 2"), 8.0);
        assert_eq!(ev("(1 + 2) * (3 + 4)"), 21.0);
        assert_eq!(ev("  7 * 6  "), 42.0);
        assert_eq!(ev("1.5e3 + .5"), 1500.5);
    }

    #[test]
    fn operator_aliases() {
        assert_eq!(ev("2 × 3 ÷ 4"), 1.5);
        assert_eq!(ev("2**3"), 8.0);
        assert_eq!(ev("5²"), 25.0);
        assert_eq!(ev("2¹⁰"), 1024.0);
        assert_eq!(ev("10 − 3"), 7.0);
    }

    #[test]
    fn mixed_expressions() {
        assert_eq!(ev("2sin(90) + 3!"), 8.0);
        assert_eq!(ev("sqrt(3² + 4²)"), 5.0);
        assert_eq!(ev("max(2, 3)! - 2^-1"), 5.5);
        assert_eq!(ev("-(2 + 3)²"), -25.0);
        assert!(close(ev("2pi * 10"), 20.0 * std::f64::consts::PI));
        assert!(close(ev("log(100) + ln(e²)"), 4.0));
    }

    #[test]
    fn launcher_style() {
        assert!(close(ev("49.99 + 19%"), 59.4881)); // add VAT
        assert_eq!(ev("120 - 25%"), 90.0); // discount
        assert_eq!(ev("15% * 80"), 12.0); // tip
        assert_eq!(ev("1024 / 8"), 128.0);
        assert_eq!(ev("17 % 5"), 2.0);
    }

    #[test]
    fn errors_surface() {
        assert_eq!(eval_str("1 +"), Err(Error::UnexpectedEnd));
        assert_eq!(eval_str("(1 + 2"), Err(Error::MissingParen));
        assert_eq!(eval_str("2 $ 3"), Err(Error::UnexpectedChar('$', 2)));
        assert_eq!(eval_str("5 / 0"), Err(Error::NotFinite));
        assert_eq!(eval_str("x + 1"), Err(Error::UnknownVariable("x".into())));
        // plain text the launcher may pass in
        assert!(eval_str("firefox").is_err());
    }
}
