//! `elementary` – a small math expression evaluator (meval replacement).
//!
//! ```
//! assert_eq!(elementary::eval_str("200 + 10%").unwrap(), 220.0);
//! assert_eq!(elementary::eval_str("2sin(90) + 3!").unwrap(), 8.0);
//! ```

mod error;
pub mod eval;
pub mod lexer;
pub mod parser;

pub use error::Error;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(eval_str("  "), Err(Error::Empty));
    }
}
