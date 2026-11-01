use std::{borrow::Cow, fmt, str::FromStr};

use serde_json::Value;

use crate::{
    error::{Error, ErrorKind},
    syntax::{self, RawSegment},
};

/// One step of a `Path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Object key: `temp_C`
    Key(Cow<'static, str>),
    /// Array index: `0`
    Index(usize),
    /// Every element of an array: `*`
    Each,
}

/// A parsed path like `weather.*.hourly.0.tempC`.
///
/// Segments are separated by `.`; `*` selects every array element and an
/// all-digit segment is an index.
///
/// Built either at compile time by `#[derive(Extract)]` (borrowed, `const`)
/// or at runtime with `Path::parse`](owned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    segments: Cow<'static, [Segment]>,
}

/// A value found by `Path::select`, together with its concrete location
/// (wildcards replaced by the actual index), used for error messages.
#[derive(Debug, Clone)]
pub struct Match<'v> {
    /// Borrowed from the document, or owned after a transform (e.g. `trim`).
    pub value: Cow<'v, Value>,
    pub at: String,
}

impl Path {
    /// A path from fixed segments, usable in `const`. Used by the derive macro.
    pub const fn from_static(segments: &'static [Segment]) -> Self {
        Self {
            segments: Cow::Borrowed(segments),
        }
    }

    pub fn parse(raw: &str) -> Result<Self, Error> {
        let segments = syntax::parse(raw)
            .map_err(|why| Error::new(raw, ErrorKind::InvalidPath(why)))?
            .into_iter()
            .map(|s| match s {
                RawSegment::Key(k) => Segment::Key(Cow::Owned(k.to_string())),
                RawSegment::Index(i) => Segment::Index(i),
                RawSegment::Each => Segment::Each,
            })
            .collect::<Vec<_>>();
        Ok(Self {
            segments: Cow::Owned(segments),
        })
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Whether this path can yield more than one value.
    pub fn is_multi(&self) -> bool {
        self.segments.contains(&Segment::Each)
    }

    /// All values this path points to, relative to `root` located at `base`.
    ///
    /// Lenient descent: a key applied to a one-element array looks inside that
    /// element, so `current_condition.temp_C` works for `[{ "temp_C": .. }]`.
    pub fn select<'v>(&self, root: &'v Value, base: &str) -> Vec<Match<'v>> {
        let mut out = Vec::new();
        walk(root, &self.segments, base.to_string(), &mut out);
        out
    }
}

fn join(base: &str, step: impl fmt::Display) -> String {
    if base.is_empty() {
        step.to_string()
    } else {
        format!("{base}.{step}")
    }
}

fn walk<'v>(value: &'v Value, segs: &[Segment], at: String, out: &mut Vec<Match<'v>>) {
    let Some((seg, rest)) = segs.split_first() else {
        out.push(Match {
            value: Cow::Borrowed(value),
            at,
        });
        return;
    };
    match (seg, value) {
        (Segment::Key(k), Value::Object(map)) => {
            if let Some(v) = map.get(&**k) {
                walk(v, rest, join(&at, k), out);
            }
        }
        // Lenient: step through single-element wrapper arrays.
        (Segment::Key(_), Value::Array(arr)) if arr.len() == 1 => {
            walk(&arr[0], segs, join(&at, 0), out);
        }
        (Segment::Index(i), Value::Array(arr)) => {
            if let Some(v) = arr.get(*i) {
                walk(v, rest, join(&at, i), out);
            }
        }
        (Segment::Each, Value::Array(arr)) => {
            for (i, v) in arr.iter().enumerate() {
                walk(v, rest, join(&at, i), out);
            }
        }
        // `*` on a single value: treat it as a list of one.
        (Segment::Each, v) => walk(v, rest, at, out),
        _ => {}
    }
}

impl FromStr for Path {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            match seg {
                Segment::Key(k) => f.write_str(k)?,
                Segment::Index(n) => write!(f, "{n}")?,
                Segment::Each => f.write_str("*")?,
            }
        }
        Ok(())
    }
}
