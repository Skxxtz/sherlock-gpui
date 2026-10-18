use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Empty,
    UnexpectedChar(char, usize),
    InvalidNumber(String, usize),
    UnexpectedToken(usize),
    UnexpectedEnd,
    MissingParen,
    UnknownVariable(String),
    UnknownFunction(String),
    WrongArgCount {
        name: String,
        expected: &'static str,
        got: usize,
    },
    /// Input outside a function's domain, e.g. `(-1)!` or `tan(90)`.
    Domain(&'static str),
    /// Result is NaN or infinite, e.g. `1/0`, `sqrt(-1)`.
    NotFinite,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty expression"),
            Error::UnexpectedChar(c, p) => write!(f, "unexpected '{c}' at {p}"),
            Error::InvalidNumber(s, p) => write!(f, "invalid number '{s}' at {p}"),
            Error::UnexpectedToken(p) => write!(f, "unexpected token at {p}"),
            Error::UnexpectedEnd => write!(f, "unexpected end of expression"),
            Error::MissingParen => write!(f, "missing closing parenthesis"),
            Error::UnknownVariable(v) => write!(f, "unknown variable '{v}'"),
            Error::UnknownFunction(n) => write!(f, "unknown function '{n}'"),
            Error::WrongArgCount {
                name,
                expected,
                got,
            } => {
                write!(f, "'{name}' takes {expected} argument(s), got {got}")
            }
            Error::Domain(what) => write!(f, "{what}"),
            Error::NotFinite => write!(f, "result is not a finite number"),
        }
    }
}

impl std::error::Error for Error {}
