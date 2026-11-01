use std::{borrow::Cow, fmt::Display};

use serde_json::Value;

use crate::{
    error::{Error, ErrorKind},
    extract::{Extract, unwrap},
    path::{Match, Path},
};

/// Per-field options. Built as a `const` by the derive macro.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Opts {
    /// Trim whitespace from every string in the matched value.
    pub trim: bool,
    /// Strings that mean "no value" (e.g. `"Nothing"`). A match equal to
    /// one of these is dropped, so `Option` gives `None`, `Vec` skips it and a
    /// plain field is missing (and falls back to `default` if set).
    /// Nested occurrences become `null`.
    pub none_if: &'static [&'static str],
}

impl Opts {
    pub const NONE: Opts = Opts {
        trim: false,
        none_if: &[],
    };

    fn is_none(&self) -> bool {
        !self.trim && self.none_if.is_empty()
    }

    fn is_sentinel(&self, v: &Value) -> bool {
        matches!(unwrap(v), Value::String(s) if self.none_if.contains(&s.trim()))
    }

    fn clean(&self, v: &Value) -> Value {
        match v {
            _ if self.is_sentinel(v) => Value::Null,
            Value::String(s) if self.trim => Value::String(s.trim().to_string()),
            Value::Array(a) => Value::Array(a.iter().map(|v| self.clean(v)).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, v)| (k.clone(), self.clean(v))).collect())
            }
            v => v.clone(),
        }
    }
}

fn location(base: &str, path: &Path) -> String {
    match (base.is_empty(), path.segments().is_empty()) {
        (true, _) => path.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}.{path}"),
    }
}

/// Select `path` and apply `opts`. Returns the matches and the field location.
fn prepare<'v>(value: &'v Value, base: &str, path: &Path, opts: &Opts) -> (Vec<Match<'v>>, String) {
    let mut matches = path.select(value, base);
    if !opts.is_none() {
        matches.retain(|m| !opts.is_sentinel(&m.value));
        for m in &mut matches {
            m.value = Cow::Owned(opts.clean(&m.value));
        }
    }
    (matches, location(base, path))
}

/// Extract the value at `path` below `value` (which lives at `base`).
pub fn field<T: Extract>(value: &Value, base: &str, path: &Path) -> Result<T, Error> {
    field_opts(value, base, path, &Opts::NONE)
}

/// Like [`field`], with options applied first.
pub fn field_opts<T: Extract>(
    value: &Value,
    base: &str,
    path: &Path,
    opts: &Opts,
) -> Result<T, Error> {
    let (matches, at) = prepare(value, base, path, opts);
    T::from_matches(matches, &at, path.is_multi())
}

/// Custom conversions: `#[at("time", with = parse_time)]`.
///
/// The function takes any `Extract` type and returns a `Result`, e.g.
/// `fn parse_time(raw: u16) -> Result<NaiveTime, String>`. Its input type is
/// extracted leniently first, so it never has to deal with raw JSON.
/// `one` / `opt` / `many` match a field of type `U`, `Option<U>`, `Vec<U>`.
pub mod with {
    use super::*;

    fn apply<T: Extract, U, E: Display>(
        m: &Match<'_>,
        f: &impl Fn(T) -> Result<U, E>,
    ) -> Result<U, Error> {
        f(T::extract(&m.value, &m.at)?).map_err(|e| Error::custom(&m.at, e))
    }

    pub fn one<T: Extract, U, E: Display>(
        value: &Value,
        base: &str,
        path: &Path,
        opts: &Opts,
        f: impl Fn(T) -> Result<U, E>,
    ) -> Result<U, Error> {
        let (matches, at) = prepare(value, base, path, opts);
        match matches.as_slice() {
            [] => Err(Error::new(at, ErrorKind::Missing)),
            [m] => apply(m, &f),
            ms => Err(Error::new(at, ErrorKind::Ambiguous(ms.len()))),
        }
    }

    pub fn opt<T: Extract, U, E: Display>(
        value: &Value,
        base: &str,
        path: &Path,
        opts: &Opts,
        f: impl Fn(T) -> Result<U, E>,
    ) -> Result<Option<U>, Error> {
        match one(value, base, path, opts, f) {
            Err(Error {
                kind: ErrorKind::Missing,
                ..
            }) => Ok(None),
            r => r.map(Some),
        }
    }

    pub fn many<T: Extract, U, E: Display>(
        value: &Value,
        base: &str,
        path: &Path,
        opts: &Opts,
        f: impl Fn(T) -> Result<U, E>,
    ) -> Result<Vec<U>, Error> {
        let (matches, at) = prepare(value, base, path, opts);
        match (path.is_multi(), matches.as_slice()) {
            (_, []) => Ok(Vec::new()),
            (true, ms) => ms.iter().map(|m| apply(m, &f)).collect(),
            // Plain path to an array: convert each element.
            (false, [m]) => match m.value.as_ref() {
                Value::Array(arr) => arr
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let m = Match {
                            value: Cow::Borrowed(v),
                            at: format!("{}.{i}", m.at),
                        };
                        apply(&m, &f)
                    })
                    .collect(),
                _ => apply(m, &f).map(|u| vec![u]),
            },
            (false, ms) => Err(Error::new(at, ErrorKind::Ambiguous(ms.len()))),
        }
    }
}
