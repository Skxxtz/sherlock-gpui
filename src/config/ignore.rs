use std::io::{self, Write};

use glob::Pattern;

use crate::{
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{ErrorKind, FileAction},
        },
        files::read_lines,
    },
};

pub struct SherlockIgnore;

impl SherlockIgnore {
    #[inline(always)]
    pub fn load(path: &std::path::Path) -> Result<Vec<Pattern>, SherlockMessage> {
        match read_lines(path) {
            Ok(lines) => Ok(lines
                .map_while(Result::ok)
                .filter_map(|l| Pattern::new(&l.to_lowercase()).ok())
                .collect()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(sherlock_msg!(
                Warning,
                ErrorKind::File(FileAction::Read, path.to_path_buf()),
                e
            )),
        }
    }

    pub fn add_entry(path: &std::path::Path, entry: &str) -> Result<(), SherlockMessage> {
        let write_err = |e: std::io::Error| {
            sherlock_msg!(
                Error,
                ErrorKind::File(FileAction::Write, path.into()),
                e.to_string()
            )
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(write_err)?;
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(write_err)?;

        writeln!(file, "{entry}").map_err(write_err)?;
        Ok(())
    }
}
