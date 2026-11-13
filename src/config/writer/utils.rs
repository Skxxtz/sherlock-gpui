use std::{fmt::Display, path::Path};

use crate::{
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{FileAction, SherlockErrorType},
    },
};

pub fn is_json(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "json")
}

pub fn read_or_empty(path: &Path) -> Result<String, SherlockMessage> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(file_err(FileAction::Read, path, &e)),
    }
}

pub fn file_err(action: FileAction, path: &Path, e: &dyn Display) -> SherlockMessage {
    sherlock_msg!(
        Warning,
        SherlockErrorType::FileError(action, path.to_path_buf()),
        e.to_string()
    )
}

pub fn ser_err(e: impl Display) -> SherlockMessage {
    sherlock_msg!(
        Warning,
        SherlockErrorType::SerializationError,
        e.to_string()
    )
}
