use std::{fmt::Display, path::Path};

use crate::{
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{ErrorKind, FsOp},
    },
};

pub fn is_json(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "json")
}

pub fn read_or_empty(path: &Path) -> Result<String, SherlockMessage> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(file_err(FsOp::Read, path, &e)),
    }
}

pub fn file_err(action: FsOp, path: &Path, e: &dyn Display) -> SherlockMessage {
    sherlock_msg!(ErrorKind::fs(action, path), e.to_string())
}
