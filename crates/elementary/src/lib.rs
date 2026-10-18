//! `elementary` – a small math expression evaluator (meval replacement).

mod error;
pub mod lexer;

pub use error::Error;

/// Parse and evaluate an expression string.
pub fn eval_str<S: AsRef<str>>(expr: S) -> Result<f64, Error> {
    let expr = expr.as_ref().trim();
    if expr.is_empty() {
        return Err(Error::Empty);
    }
    Err(Error::Unimplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(eval_str("  "), Err(Error::Empty));
    }
}
