use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use crate::ui::model::file::{backends::command::CommandFactory, filter::FileFilter};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RgFactory;

impl CommandFactory for RgFactory {
    const BINARY_NAME: &'static str = "rg";
    const HANDLES_FILTERING: bool = true;

    fn args<'a>(
        &self,
        query: &'a str,
        filter: &'a FileFilter,
        paths: &'a [PathBuf],
    ) -> impl Iterator<Item = Cow<'a, OsStr>> {
        let flags = ["-l", "-F", "-S"]
            .into_iter()
            .map(|f| Cow::Borrowed(OsStr::new(f)));

        let globs = filter
            .pushdown_exts()
            .into_iter()
            .flatten()
            .flat_map(|ext| {
                [
                    Cow::Borrowed(OsStr::new("--iglob")),
                    Cow::Owned(OsString::from(format!("*.{ext}"))),
                ]
            });

        flags
            .chain(globs)
            .chain(
                ["--", query]
                    .into_iter()
                    .map(|s| Cow::Borrowed(OsStr::new(s))),
            )
            .chain(paths.iter().map(|p| Cow::Borrowed(p.as_os_str())))
    }
}
