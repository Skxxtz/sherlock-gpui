use std::fmt;

/// An extraction error, always tagged with the concrete location it happened at
/// (e.g. `weather.2.hourly.4.chanceofrain`).
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub at: String,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Input is not valid JSON.
    Json(String),
    /// The path string itself is malformed.
    InvalidPath(&'static str),
    /// Nothing at this path.
    Missing,
    /// A single value was expected but the path matched several.
    Ambiguous(usize),
    /// Wrong JSON type (e.g. object where a number was expected).
    Type {
        expected: &'static str,
        found: &'static str,
    },
    /// Right type, but the content didn't parse (e.g. `"n/a"` as a number).
    Parse {
        expected: &'static str,
        value: String,
    },
    /// Anything raised by user code / transforms.
    Custom(String),
}

impl Error {
    pub fn new(at: impl Into<String>, kind: ErrorKind) -> Self {
        Self {
            at: at.into(),
            kind,
        }
    }
    pub fn custom(at: impl Into<String>, msg: impl fmt::Display) -> Self {
        Self::new(at, ErrorKind::Custom(msg.to_string()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = if self.at.is_empty() {
            "<root>"
        } else {
            &self.at
        };
        write!(f, "{at}: ")?;
        match &self.kind {
            ErrorKind::Json(e) => write!(f, "invalid json: {e}"),
            ErrorKind::InvalidPath(why) => write!(f, "invalid path: {why}"),
            ErrorKind::Missing => f.write_str("missing"),
            ErrorKind::Ambiguous(n) => write!(f, "expected one value, path matched {n}"),
            ErrorKind::Type { expected, found } => write!(f, "expected {expected}, found {found}"),
            ErrorKind::Parse { expected, value } => write!(f, "expected {expected}, got {value:?}"),
            ErrorKind::Custom(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {}
