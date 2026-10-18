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
    /// `solve` was given an expression without `=`.
    NotAnEquation,
    /// The equation has no unknown, e.g. `2 = 3`.
    NoUnknown,
    /// More than one unknown, e.g. `x + y = 3`.
    TooManyUnknowns(Vec<String>),
    NoSolution,
    /// Holds for every value, e.g. `x + 1 = 1 + x`.
    InfiniteSolutions,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty expression"),
            Error::UnexpectedChar(c, p) => write!(f, "unexpected '{c}' at {p}"),
            Error::InvalidNumber(s, p) => write!(f, "invalid number '{s}' at {p}"),
            Error::UnexpectedToken(p) => write!(f, "unexpected token at {p}"),
            Error::UnexpectedEnd => write!(f, "unexpected end of expression"),
            Error::MissingParen => write!(f, "missing closing bracket"),
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
            Error::NotAnEquation => write!(f, "not an equation (missing '=')"),
            Error::NoUnknown => write!(f, "equation has no unknown"),
            Error::TooManyUnknowns(v) => write!(f, "too many unknowns: {}", v.join(", ")),
            Error::NoSolution => write!(f, "no solution found"),
            Error::InfiniteSolutions => write!(f, "true for every value"),
        }
    }
}

impl std::error::Error for Error {}
