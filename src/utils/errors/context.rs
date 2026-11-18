use std::{fmt::Display, panic::Location};

use gpui::SharedString;

use super::{ErrorKind, Format, PathRef, Pos, SherlockMessage, SherlockMessageLevel};

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

    /// A parse error in the file at `path`, with the parser's position if it reports one.
    fn in_file(self, path: impl Into<PathRef>) -> Result<T, SherlockMessage>
    where
        E: ParseError;

    /// Like `in_file`, with the parsed text: parsers that report byte offsets need it.
    fn in_file_text(self, path: impl Into<PathRef>, text: &str) -> Result<T, SherlockMessage>
    where
        E: ParseError;
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

    #[track_caller]
    fn in_file(self, path: impl Into<PathRef>) -> Result<T, SherlockMessage>
    where
        E: ParseError,
    {
        let loc = Location::caller();
        self.map_err(|e| parse_file_error(e, path.into(), None, loc))
    }

    #[track_caller]
    fn in_file_text(self, path: impl Into<PathRef>, text: &str) -> Result<T, SherlockMessage>
    where
        E: ParseError,
    {
        let loc = Location::caller();
        self.map_err(|e| parse_file_error(e, path.into(), Some(text), loc))
    }
}

fn parse_file_error<E: ParseError>(
    e: E,
    path: PathRef,
    text: Option<&str>,
    loc: &Location<'_>,
) -> SherlockMessage {
    let kind = ErrorKind::parse_file(E::FORMAT, path, e.pos(text));
    let level = kind.level();
    SherlockMessage::at(level, kind, e.to_string().into(), loc)
}

/// A parser error that may know where in the input it happened.
pub trait ParseError: Display {
    const FORMAT: Format;
    /// The position, given the parsed text when the parser only reports byte offsets.
    fn pos(&self, text: Option<&str>) -> Option<Pos>;
}

impl ParseError for serde_json::Error {
    const FORMAT: Format = Format::Json;
    fn pos(&self, _: Option<&str>) -> Option<Pos> {
        (self.line() > 0).then(|| Pos {
            line: self.line() as u32,
            col: self.column() as u32,
        })
    }
}

impl ParseError for simd_json::Error {
    const FORMAT: Format = Format::Json;
    fn pos(&self, text: Option<&str>) -> Option<Pos> {
        text.map(|t| Pos::from_offset(t, self.index()))
    }
}

impl ParseError for toml::de::Error {
    const FORMAT: Format = Format::Toml;
    fn pos(&self, text: Option<&str>) -> Option<Pos> {
        Some(Pos::from_offset(text?, self.span()?.start))
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
            let level = kind.level();
            SherlockMessage::at(level, kind, SharedString::default(), loc)
        })
    }

    #[track_caller]
    fn or_kind_with(self, kind: impl FnOnce() -> ErrorKind) -> Result<T, SherlockMessage> {
        let loc = Location::caller();
        self.ok_or_else(|| {
            let kind = kind();
            let level = kind.level();
            SherlockMessage::at(level, kind, SharedString::default(), loc)
        })
    }
}

impl ParseError for toml_edit::TomlError {
    const FORMAT: Format = Format::Toml;
    fn pos(&self, text: Option<&str>) -> Option<Pos> {
        Some(Pos::from_offset(text?, self.span()?.start))
    }
}
