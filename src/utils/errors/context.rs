use std::{fmt::Display, panic::Location};

use gpui::SharedString;

use super::{ErrorKind, SherlockMessage, SherlockMessageLevel};

#[allow(dead_code)]
pub trait ResultExt<T, E> {
    /// Names what failed. The error becomes the cause. Level: warning.
    fn or_kind(self, kind: ErrorKind) -> Result<T, SherlockMessage>
    where
        E: Display + 'static;

    /// Like `or_kind`, building the kind only on error.
    fn or_kind_with(self, kind: impl FnOnce() -> ErrorKind) -> Result<T, SherlockMessage>
    where
        E: Display + 'static;

    /// Adds what was being done. The error must convert into a `SherlockMessage`.
    fn context(self, what: impl Into<SharedString>) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>;

    /// Like `context`, building the text only on error.
    fn context_with<S: Into<SharedString>>(
        self,
        what: impl FnOnce() -> S,
    ) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>;

    /// Sets the level of the error.
    fn level(self, level: SherlockMessageLevel) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>;

    /// The value, or `None` after logging the error.
    fn ok_or_log(self) -> Option<T>
    where
        SherlockMessage: From<E>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    #[track_caller]
    fn or_kind(self, kind: ErrorKind) -> Result<T, SherlockMessage>
    where
        E: Display + 'static,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(SherlockMessage::wrap(kind, e)),
        }
    }

    #[track_caller]
    fn or_kind_with(self, kind: impl FnOnce() -> ErrorKind) -> Result<T, SherlockMessage>
    where
        E: Display + 'static,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(SherlockMessage::wrap(kind(), e)),
        }
    }

    #[track_caller]
    fn context(self, what: impl Into<SharedString>) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(SherlockMessage::from(e).context(what)),
        }
    }

    #[track_caller]
    fn context_with<S: Into<SharedString>>(
        self,
        what: impl FnOnce() -> S,
    ) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(SherlockMessage::from(e).context(what())),
        }
    }

    #[track_caller]
    fn level(self, level: SherlockMessageLevel) -> Result<T, SherlockMessage>
    where
        SherlockMessage: From<E>,
    {
        self.map_err(|e| SherlockMessage::from(e).with_level(level))
    }

    #[track_caller]
    fn ok_or_log(self) -> Option<T>
    where
        SherlockMessage: From<E>,
    {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                SherlockMessage::from(e).log();
                None
            }
        }
    }
}

#[allow(dead_code)]
pub trait OptionExt<T> {
    /// `None` becomes an error of `kind`. Level: warning.
    fn or_kind(self, kind: ErrorKind) -> Result<T, SherlockMessage>;

    /// Like `or_kind`, building the kind only on `None`.
    fn or_kind_with(self, kind: impl FnOnce() -> ErrorKind) -> Result<T, SherlockMessage>;
}

impl<T> OptionExt<T> for Option<T> {
    #[track_caller]
    fn or_kind(self, kind: ErrorKind) -> Result<T, SherlockMessage> {
        let loc = Location::caller();
        self.ok_or_else(|| {
            SherlockMessage::at(
                SherlockMessageLevel::Warning,
                kind,
                SharedString::default(),
                loc,
            )
        })
    }

    #[track_caller]
    fn or_kind_with(self, kind: impl FnOnce() -> ErrorKind) -> Result<T, SherlockMessage> {
        let loc = Location::caller();
        self.ok_or_else(|| {
            SherlockMessage::at(
                SherlockMessageLevel::Warning,
                kind(),
                SharedString::default(),
                loc,
            )
        })
    }
}
