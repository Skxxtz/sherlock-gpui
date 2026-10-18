use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Empty,
    Unimplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty expression"),
            Error::Unimplemented => write!(f, "not implemented yet"),
        }
    }
}

impl std::error::Error for Error {}
