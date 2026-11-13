use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, IntoStaticStr};

/// What failed. The message is the headline; details go into the error's context and cause.
///
/// Adding a kind: one variant with its `#[error]` message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// A branch the code logic says can't be reached.
    #[error("Unreachable code was reached")]
    Unreachable,
    /// The provided data is malformed or incomplete.
    #[error("Invalid data")]
    InvalidData,
    /// The launcher doesn't have this functionality.
    #[error("Invalid launcher")]
    InvalidLauncher,
    /// The requested action isn't defined.
    #[error("Invalid action")]
    InvalidAction,
    /// The active launcher doesn't support this function.
    #[error("Invalid function")]
    InvalidFunction,
    /// A preview message from the debug launcher.
    #[error("Preview")]
    Preview,

    /// Turning internal data into a string or bytes failed.
    #[error("Failed to serialize")]
    Serialize,
    /// Reading external data (what) failed.
    #[error("Failed to parse {0}")]
    Parse(String),
    /// The user's configuration is wrong.
    #[error("Config error: {0}")]
    Config(String),

    /// A general I/O failure.
    #[error("I/O error")]
    Io,
    #[error("Failed to {action} file \"{path}\"", action = .0, path = .1.display())]
    File(FileAction, PathBuf),
    #[error("Failed to {action} directory \"{path}\"", action = .0, path = .1.display())]
    Dir(DirAction, PathBuf),
    /// Running an external command failed.
    #[error("Command failed: {0}")]
    Command(String),
    /// An environment variable is missing or malformed.
    #[error("Failed to read \"{0}\"")]
    Env(String),
    #[error("DBus failed to {0} ({1})")]
    DBus(DBusAction, String),
    #[error("Failed to {0} socket")]
    Socket(SocketAction),
    #[error("Network failure during {0} from \"{1}\"")]
    Network(NetworkAction, String),
    #[error("Database failure during {0}")]
    Database(DbAction),

    /// The plugin system failed (action, plugin name).
    #[error("Plugin `{1}` failed to {0}")]
    Plugin(PluginAction, String),
}

impl ErrorKind {
    /// The kind's name, for headers and logs.
    pub fn name(&self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum PluginAction {
    Load,
    TileInit,
    Live,
    Callback,
    Query,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum FileAction {
    Read,
    Write,
    Parse,
    Remove,
    Find,
    Create,
    Rename,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum DirAction {
    Read,
    Create,
    Remove,
    Find,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum DBusAction {
    Connect,
    Construct,
    Send,
    Call,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum SocketAction {
    Close,
    Connect,
    Write,
    Read,
    EoF,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum NetworkAction {
    Get,
    Post,
    Put,
    Update,
    Delete,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum DbAction {
    Connect,
    Query,
    Initialize,
    Migrate,
    Transaction,
}
