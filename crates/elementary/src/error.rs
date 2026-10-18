use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Empty,
    UnexpectedChar(char, usize),
    InvalidNumber(String, usize),
    Unimplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty expression"),
            Error::UnexpectedChar(c, p) => write!(f, "unexpected '{c}' at {p}"),
            Error::InvalidNumber(s, p) => write!(f, "invalid number '{s}' at {p}"),
            Error::Unimplemented => write!(f, "not implemented yet"),
        }
    }
}

impl std::error::Error for Error {}
