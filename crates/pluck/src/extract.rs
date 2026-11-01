use serde_json::Value;

use crate::{
    error::{Error, ErrorKind},
    path::Match,
};

/// A type that can be pulled out of a JSON value.
///
/// Scalars are lenient: numeric strings become numbers, and wrappers like
/// `[x]` or `{ "value": x }` are unwrapped first.
pub trait Extract: Sized {
    /// Convert one value located at `at`.
    fn extract(value: &Value, at: &str) -> Result<Self, Error>;

    /// Convert everything a path matched; `multi` is true for wildcard paths.
    /// Default: exactly one match. `Option` and `Vec` override this.
    fn from_matches(matches: Vec<Match<'_>>, at: &str, _multi: bool) -> Result<Self, Error> {
        match matches.len() {
            0 => Err(Error::new(at, ErrorKind::Missing)),
            1 => Self::extract(&matches[0].value, &matches[0].at),
            n => Err(Error::new(at, ErrorKind::Ambiguous(n))),
        }
    }
}

/// Strip wrapper layers: `[x]` -> `x`, `{ "value": x }` -> `x`.
pub fn unwrap(mut value: &Value) -> &Value {
    loop {
        value = match value {
            Value::Array(a) if a.len() == 1 => &a[0],
            Value::Object(o) if o.len() == 1 && o.contains_key("value") => &o["value"],
            _ => return value,
        };
    }
}

pub(crate) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_err(at: &str, expected: &'static str, found: &Value) -> Error {
    Error::new(
        at,
        ErrorKind::Type {
            expected,
            found: type_name(found),
        },
    )
}

fn parse_err(at: &str, expected: &'static str, value: &str) -> Error {
    Error::new(
        at,
        ErrorKind::Parse {
            expected,
            value: value.to_string(),
        },
    )
}

impl Extract for Value {
    fn extract(value: &Value, _at: &str) -> Result<Self, Error> {
        Ok(value.clone())
    }
}

impl Extract for String {
    fn extract(value: &Value, at: &str) -> Result<Self, Error> {
        match unwrap(value) {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            v => Err(type_err(at, "string", v)),
        }
    }
}

impl Extract for bool {
    fn extract(value: &Value, at: &str) -> Result<Self, Error> {
        match unwrap(value) {
            Value::Bool(b) => Ok(*b),
            Value::Number(n) => Ok(n.as_f64().is_some_and(|n| n != 0.0)),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(true),
                "false" | "no" | "0" => Ok(false),
                _ => Err(parse_err(at, "bool", s)),
            },
            v => Err(type_err(at, "bool", v)),
        }
    }
}

macro_rules! int {
    ($($t:ty),*) => {$(
        impl Extract for $t {
            fn extract(value: &Value, at: &str) -> Result<Self, Error> {
                const NAME: &str = stringify!($t);
                match unwrap(value) {
                    Value::Number(n) => n
                        .as_i64()
                        .and_then(|n| <$t>::try_from(n).ok())
                        .or_else(|| n.as_u64().and_then(|n| <$t>::try_from(n).ok()))
                        .ok_or_else(|| parse_err(at, NAME, &n.to_string())),
                    Value::String(s) => s.trim().parse().map_err(|_| parse_err(at, NAME, s)),
                    v => Err(type_err(at, NAME, v)),
                }
            }
        }
    )*};
}
int!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);

macro_rules! float {
    ($($t:ty),*) => {$(
        impl Extract for $t {
            fn extract(value: &Value, at: &str) -> Result<Self, Error> {
                const NAME: &str = stringify!($t);
                match unwrap(value) {
                    Value::Number(n) => n
                        .as_f64()
                        .map(|n| n as $t)
                        .ok_or_else(|| parse_err(at, NAME, &n.to_string())),
                    Value::String(s) => s.trim().parse().map_err(|_| parse_err(at, NAME, s)),
                    v => Err(type_err(at, NAME, v)),
                }
            }
        }
    )*};
}
float!(f32, f64);

impl<T: Extract> Extract for Option<T> {
    fn extract(value: &Value, at: &str) -> Result<Self, Error> {
        match value {
            Value::Null => Ok(None),
            v => T::extract(v, at).map(Some),
        }
    }
    fn from_matches(matches: Vec<Match<'_>>, at: &str, multi: bool) -> Result<Self, Error> {
        if matches.is_empty() {
            return Ok(None);
        }
        T::from_matches(matches, at, multi).map(Some)
    }
}

impl<T: Extract> Extract for Vec<T> {
    /// A single value that is an array: convert each element.
    /// Anything else becomes a list of one.
    fn extract(value: &Value, at: &str) -> Result<Self, Error> {
        match value {
            Value::Array(arr) => arr
                .iter()
                .enumerate()
                .map(|(i, v)| T::extract(v, &format!("{at}.{i}")))
                .collect(),
            v => T::extract(v, at).map(|t| vec![t]),
        }
    }
    /// Wildcard path: one element per match. Plain path: the matched array.
    /// Nothing matched → empty list.
    fn from_matches(matches: Vec<Match<'_>>, at: &str, multi: bool) -> Result<Self, Error> {
        match (multi, matches.as_slice()) {
            (_, []) => Ok(Vec::new()),
            (true, ms) => ms.iter().map(|m| T::extract(&m.value, &m.at)).collect(),
            (false, [m]) => Self::extract(&m.value, &m.at),
            (false, ms) => Err(Error::new(at, ErrorKind::Ambiguous(ms.len()))),
        }
    }
}
