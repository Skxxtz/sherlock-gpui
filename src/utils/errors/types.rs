use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::SharedString;
use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, IntoStaticStr};

use super::SherlockMessageLevel;

/// Declares every [`ErrorKind`] in one table. Each row generates the variant, its message
/// (`thiserror`), stable code, default level, category, optional hint and catalog entry.
///
/// ```ignore
/// /// docs, shown by `--explain`
/// Variant(Fields..) => "S0000", Level, Category, ("message {0}"), hint: "what to do";
/// ```
macro_rules! error_kinds {
    ($(
        $(#[doc = $doc:literal])*
        $variant:ident $( ( $($tty:ty),+ $(,)? ) )? $( { $($field:ident : $fty:ty),+ $(,)? } )?
            => $code:literal, $level:ident, $cat:ident, ($($msg:tt)+) $(, hint: $hint:literal)? ;
    )+) => {
        /// What failed. The message is the headline; details go into the error's context and
        /// cause. Kinds are declared in the `error_kinds!` table: one row per kind.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr, thiserror::Error)]
        #[serde(rename_all = "snake_case")]
        pub enum ErrorKind {
            $(
                $(#[doc = $doc])*
                #[error($($msg)+)]
                $variant $( ( $($tty),+ ) )? $( { $($field: $fty),+ } )?,
            )+
        }

        impl ErrorKind {
            /// Every kind, for `--explain` and tests.
            pub const CATALOG: &[KindInfo] = &[$(
                KindInfo {
                    code: $code,
                    name: stringify!($variant),
                    level: SherlockMessageLevel::$level,
                    category: Category::$cat,
                    doc: concat!($($doc, "\n"),*),
                    hint: error_kinds!(@opt $($hint)?),
                },
            )+];

            /// Stable code, e.g. `S0102`. Never reuse a retired code.
            pub fn code(&self) -> &'static str {
                match self { $(Self::$variant { .. } => $code,)+ }
            }

            /// The level a message of this kind gets unless overridden.
            pub fn level(&self) -> SherlockMessageLevel {
                match self { $(Self::$variant { .. } => SherlockMessageLevel::$level,)+ }
            }

            pub fn category(&self) -> Category {
                match self { $(Self::$variant { .. } => Category::$cat,)+ }
            }

            /// What the user can do about it.
            pub fn hint(&self) -> Option<&'static str> {
                match self { $(Self::$variant { .. } => error_kinds!(@opt $($hint)?),)+ }
            }
        }
    };
    (@opt) => { None };
    (@opt $h:literal) => { Some($h) };
}

error_kinds! {
    // ── internal: S00xx ─────────────────────────────────────────────────────────────────────
    /// An invariant the code relies on was broken. Always a bug in Sherlock.
    Bug(SharedString)
        => "S0001", Error, Internal, ("Internal error: {0}"),
        hint: "this is a bug in Sherlock: copy the report (ctrl-c) and open an issue";
    /// The active launcher, action or function isn't supported here.
    Unsupported(Feature)
        => "S0002", Warning, Internal, ("Unsupported {0}");
    /// A test message from the debug launcher.
    Demo
        => "S0003", Info, Internal, ("Test message");

    // ── data: S01xx ─────────────────────────────────────────────────────────────────────────
    /// Data is malformed or incomplete. The cause says how.
    InvalidData
        => "S0101", Warning, Data, ("Invalid data");
    /// Data that isn't a file (a response, a socket message, …) couldn't be parsed.
    Parse { format: Format, what: SharedString }
        => "S0102", Warning, Data, ("Failed to parse {format} ({what})");
    /// A file couldn't be parsed. Carries the position when the parser reports one.
    ParseFile { format: Format, path: PathRef, pos: Option<Pos> }
        => "S0103", Warning, Data, ("Failed to parse {format} file \"{path}\"{}", fmt_pos(.pos)),
        hint: "fix the syntax at the marked position (enter opens the file)";
    /// Internal data couldn't be turned into text or bytes.
    Serialize(Format)
        => "S0104", Warning, Data, ("Failed to serialize {0}");

    // ── config: S02xx ───────────────────────────────────────────────────────────────────────
    /// The configuration is wrong.
    Config(SharedString)
        => "S0201", Warning, Config, ("Config error: {0}"),
        hint: "check your Sherlock config (default: ~/.config/sherlock/)";
    /// An environment variable is missing or malformed.
    Env(SharedString)
        => "S0202", Warning, Config, ("${0} is not set"),
        hint: "set it in your shell or session environment";
    /// Something Sherlock looks up on the system wasn't found.
    Missing(SharedString)
        => "S0203", Warning, Config, ("Not found: {0}");

    // ── system: S03xx ───────────────────────────────────────────────────────────────────────
    /// A general I/O failure.
    Io
        => "S0301", Warning, System, ("I/O error");
    /// A file or directory operation failed.
    Fs { op: FsOp, path: PathRef }
        => "S0302", Warning, System, ("Failed to {op} \"{path}\"");
    /// Running an external command failed.
    Command(SharedString)
        => "S0303", Warning, System, ("Command failed: {0}");
    /// A database operation failed.
    Database(DbOp)
        => "S0304", Warning, System, ("Database {0} failed");

    // ── ipc: S04xx ──────────────────────────────────────────────────────────────────────────
    /// A socket operation failed.
    Ipc(IoOp)
        => "S0401", Warning, Ipc, ("Socket {0} failed");
    /// The peer closed the connection.
    Closed
        => "S0402", Warning, Ipc, ("Connection closed");
    /// A D-Bus operation failed.
    DBus { op: DBusOp, target: SharedString }
        => "S0403", Warning, Ipc, ("D-Bus {op} failed ({target})");

    // ── network: S05xx ──────────────────────────────────────────────────────────────────────
    /// An HTTP request failed.
    Http { method: HttpMethod, target: SharedString }
        => "S0501", Warning, Net, ("HTTP {method} to \"{target}\" failed"),
        hint: "check your internet connection";

    // ── plugins: S06xx ──────────────────────────────────────────────────────────────────────
    /// The plugin system failed.
    Plugin { op: PluginOp, name: SharedString }
        => "S0601", Warning, Plugin, ("Plugin `{name}` failed to {op}");
}

/// Constructors for the common kinds.
impl ErrorKind {
    /// The kind's name, for search and logs.
    pub fn name(&self) -> &'static str {
        self.into()
    }

    /// What the UI can offer to fix it.
    pub fn remedy(&self) -> Option<Remedy> {
        match self {
            Self::ParseFile { path, pos, .. } => Some(Remedy::OpenFile(path.clone(), *pos)),
            _ => None,
        }
    }

    /// The catalog entry for a code (case-insensitive).
    pub fn explain(code: &str) -> Option<&'static KindInfo> {
        Self::CATALOG
            .iter()
            .find(|k| k.code.eq_ignore_ascii_case(code))
    }

    pub fn bug(what: impl Into<SharedString>) -> Self {
        Self::Bug(what.into())
    }
    pub fn fs(op: FsOp, path: impl Into<PathRef>) -> Self {
        Self::Fs {
            op,
            path: path.into(),
        }
    }
    pub fn parse(format: Format, what: impl Into<SharedString>) -> Self {
        Self::Parse {
            format,
            what: what.into(),
        }
    }
    pub fn parse_file(format: Format, path: impl Into<PathRef>, pos: Option<Pos>) -> Self {
        Self::ParseFile {
            format,
            path: path.into(),
            pos,
        }
    }
    pub fn config(what: impl Into<SharedString>) -> Self {
        Self::Config(what.into())
    }
    pub fn env(var: impl Into<SharedString>) -> Self {
        Self::Env(var.into())
    }
    pub fn missing(what: impl Into<SharedString>) -> Self {
        Self::Missing(what.into())
    }
    pub fn command(cmd: impl Into<SharedString>) -> Self {
        Self::Command(cmd.into())
    }
    pub fn dbus(op: DBusOp, target: impl Into<SharedString>) -> Self {
        Self::DBus {
            op,
            target: target.into(),
        }
    }
    pub fn http(method: HttpMethod, target: impl Into<SharedString>) -> Self {
        Self::Http {
            method,
            target: target.into(),
        }
    }
    pub fn plugin(op: PluginOp, name: impl Into<SharedString>) -> Self {
        Self::Plugin {
            op,
            name: name.into(),
        }
    }
}

/// A catalog row: everything the table says about one kind.
#[derive(Debug)]
pub struct KindInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub level: SherlockMessageLevel,
    pub category: Category,
    pub doc: &'static str,
    pub hint: Option<&'static str>,
}

impl Display for KindInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}  {} ({}, {:?} by default)",
            self.code, self.name, self.category, self.level
        )?;
        for line in self.doc.lines() {
            writeln!(f, "  {}", line.trim())?;
        }
        if let Some(hint) = self.hint {
            writeln!(f, "  hint: {hint}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum Category {
    Internal,
    Data,
    Config,
    System,
    Ipc,
    Net,
    Plugin,
}

/// An action the UI offers on a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Remedy {
    OpenFile(PathRef, Option<Pos>),
}

/// 1-based position in a text file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl Pos {
    /// The position of byte `offset` in `text`.
    pub fn from_offset(text: &str, offset: usize) -> Self {
        let before = &text.as_bytes()[..offset.min(text.len())];
        let line = memchr::memchr_iter(b'\n', before).count() as u32 + 1;
        let line_start = memchr::memrchr(b'\n', before).map_or(0, |i| i + 1);
        let col = String::from_utf8_lossy(&before[line_start..])
            .chars()
            .count() as u32
            + 1;
        Self { line, col }
    }
}

fn fmt_pos(pos: &Option<Pos>) -> String {
    pos.map(|p| format!(" at {}:{}", p.line, p.col))
        .unwrap_or_default()
}

/// A cheaply clonable path: `Arc<Path>` with `Display` and serde.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathRef(pub Arc<Path>);

impl Display for PathRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(f)
    }
}
impl std::ops::Deref for PathRef {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}
impl From<&Path> for PathRef {
    fn from(p: &Path) -> Self {
        Self(p.into())
    }
}
impl From<&PathBuf> for PathRef {
    fn from(p: &PathBuf) -> Self {
        Self(p.as_path().into())
    }
}
impl From<PathBuf> for PathRef {
    fn from(p: PathBuf) -> Self {
        Self(p.into())
    }
}
impl From<Arc<Path>> for PathRef {
    fn from(p: Arc<Path>) -> Self {
        Self(p)
    }
}
impl From<&Arc<Path>> for PathRef {
    fn from(p: &Arc<Path>) -> Self {
        Self(p.clone())
    }
}
impl From<&str> for PathRef {
    fn from(p: &str) -> Self {
        Self(Path::new(p).into())
    }
}
impl Serialize for PathRef {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}
impl<'de> Deserialize<'de> for PathRef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        PathBuf::deserialize(d).map(Self::from)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum Format {
    Json,
    Toml,
    Binary,
    Text,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum Feature {
    Launcher,
    Action,
    Function,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum FsOp {
    Read,
    Write,
    Create,
    Remove,
    Rename,
    Find,
    #[strum(serialize = "read directory")]
    ReadDir,
    #[strum(serialize = "create directory")]
    CreateDir,
    #[strum(serialize = "remove directory")]
    RemoveDir,
    #[strum(serialize = "find directory")]
    FindDir,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum IoOp {
    Connect,
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum DBusOp {
    Connect,
    Construct,
    Call,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum DbOp {
    Connect,
    Query,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, StrumDisplay, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum PluginOp {
    Load,
    #[strum(serialize = "init tile")]
    TileInit,
    Live,
    Callback,
    Query,
}
