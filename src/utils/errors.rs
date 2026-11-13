//! Sherlock's error type.
//!
//! A [`SherlockMessage`] has a [`level`](SherlockMessageLevel), a [`kind`](ErrorKind) (what
//! failed), a context chain (what was being done, outermost first) and a cause (the underlying
//! error text). It records where it was made, via `#[track_caller]`.
//!
//! ```ignore
//! // foreign errors: name what failed
//! fs::create_dir_all(dir).or_kind_with(|| ErrorKind::Dir(DirAction::Create, dir.into()))?;
//!
//! // common ones convert with `?` (io, toml, json)
//! let text = fs::read_to_string(path)?;
//!
//! // add what you were doing, on the way up
//! load_launchers(path).context("loading fallback launchers")?;
//!
//! // made from nothing
//! return Err(sherlock_msg!(Warning, ErrorKind::Config("unknown launcher".into()), name));
//! ```
//!
//! **Logging:** every message is logged exactly once, with its full context: when it is
//! [reported](SherlockMessage::report) (shown in the UI), or else when it is dropped. Expected
//! failures that shouldn't be logged are marked with [`SherlockMessage::quiet`].

use std::{
    any::Any,
    fmt::{Debug, Display},
    panic::Location,
};

use gpui::SharedString;
use serde::{Deserialize, Serialize};

mod context;
mod convert;
pub mod types;

#[allow(unused_imports)] // OptionExt: no callers yet
pub use context::{OptionExt, ResultExt};
pub use types::*;

/// Builds a [`SherlockMessage`] from nothing.
///
/// `sherlock_msg!(Warning, ErrorKind::Config("x".into()), cause)`. The cause is anything
/// [`Display`].
#[macro_export]
macro_rules! sherlock_msg {
    ($level:ident, $kind:expr, $cause:expr) => {
        $crate::utils::errors::SherlockMessage::new(
            $crate::utils::errors::SherlockMessageLevel::$level,
            $kind,
            $cause,
        )
    };
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SherlockMessageLevel {
    Info,
    #[default]
    Warning,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SherlockMessage {
    pub level: SherlockMessageLevel,
    pub kind: ErrorKind,
    /// What was being done, outermost first.
    pub context: Vec<SharedString>,
    /// The underlying error, as text. May be empty.
    pub cause: SharedString,
    /// `file:line` where the message was made.
    pub location: SharedString,
    /// Already logged (or deliberately not): skips the log on drop.
    #[serde(skip, default = "logged_elsewhere")]
    logged: bool,
}

/// Messages that arrive over IPC were logged by the sender.
fn logged_elsewhere() -> bool {
    true
}

impl SherlockMessage {
    /// A message of `kind` with `cause`, located at the caller.
    #[track_caller]
    pub fn new(level: SherlockMessageLevel, kind: ErrorKind, cause: impl Display) -> Self {
        Self::at(level, kind, cause.to_string().into(), Location::caller())
    }

    pub(crate) fn at(
        level: SherlockMessageLevel,
        kind: ErrorKind,
        cause: SharedString,
        loc: &Location<'_>,
    ) -> Self {
        Self {
            level,
            kind,
            context: Vec::new(),
            cause,
            location: format!("{}:{}", loc.file(), loc.line()).into(),
            logged: false,
        }
    }

    /// Wraps any error as the cause of a new message of `kind`. A wrapped `SherlockMessage`
    /// keeps its chain: its kind and context move into the new message's context.
    #[track_caller]
    pub fn wrap<E: Display + 'static>(kind: ErrorKind, error: E) -> Self {
        let loc = Location::caller();
        let mut error = error;
        if let Some(inner) = (&mut error as &mut dyn Any).downcast_mut::<SherlockMessage>() {
            inner.logged = true; // lives on in the new message
            let mut context = Vec::with_capacity(inner.context.len() + 1);
            context.push(inner.kind.to_string().into());
            context.append(&mut inner.context);
            let mut msg = Self::at(inner.level, kind, inner.cause.clone(), loc);
            msg.context = context;
            return msg;
        }
        Self::at(
            SherlockMessageLevel::Warning,
            kind,
            error.to_string().into(),
            loc,
        )
    }

    /// Adds what was being done when this failed (becomes the outermost step).
    pub fn context(mut self, what: impl Into<SharedString>) -> Self {
        self.context.insert(0, what.into());
        self
    }

    pub fn with_level(mut self, level: SherlockMessageLevel) -> Self {
        self.level = level;
        self
    }
    pub fn as_error(self) -> Self {
        self.with_level(SherlockMessageLevel::Error)
    }
    pub fn as_info(self) -> Self {
        self.with_level(SherlockMessageLevel::Info)
    }

    /// An expected failure: never logged.
    pub fn quiet(mut self) -> Self {
        self.logged = true;
        self
    }

    /// Logs the message now (once). Call where it is shown to the user.
    pub fn report(&mut self) {
        if !std::mem::replace(&mut self.logged, true) {
            self.write_log();
        }
    }

    /// Logs the message now and drops it.
    pub fn log(mut self) {
        self.report();
    }

    /// The context steps, then the cause, one per line: the details shown under the headline.
    pub fn details(&self) -> SharedString {
        let mut out = String::new();
        for (i, step) in self.context.iter().enumerate() {
            out.push_str(&"  ".repeat(i));
            out.push_str(step);
            out.push('\n');
        }
        out.push_str(&self.cause);
        out.trim_end().to_string().into()
    }

    fn write_log(&self) {
        if cfg!(test) {
            return;
        }
        let (file, line) = self
            .location
            .rsplit_once(':')
            .unwrap_or((&self.location, "0"));
        let _ = crate::sher_log!(
            format!("[{:?}] {} - {}", self.level, self.kind.name(), self),
            file,
            line.parse().unwrap_or(0)
        );
    }
}

impl Drop for SherlockMessage {
    fn drop(&mut self) {
        self.report();
    }
}

impl std::error::Error for SherlockMessage {}

/// One line: `kind: context: …: cause`.
impl Display for SherlockMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        for step in &self.context {
            write!(f, ": {step}")?;
        }
        if !self.cause.is_empty() {
            write!(f, ": {}", self.cause)?;
        }
        Ok(())
    }
}

impl Debug for SherlockMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {self} ({})", self.level, self.location)
    }
}

/// Same message: everything but the log state.
impl PartialEq for SherlockMessage {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level
            && self.kind == other.kind
            && self.context == other.context
            && self.cause == other.cause
            && self.location == other.location
    }
}

#[cfg(test)]
mod tests;
