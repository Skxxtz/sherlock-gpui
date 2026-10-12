use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

use crate::ui::model::file::backends::command::CommandFactory;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RgFactory;

impl CommandFactory for RgFactory {
    const BINARY_NAME: &'static str = "rg";
    const HANDLES_FILTERING: bool = true;

    fn args<'a>(
        &self,
        query: &'a str,
        paths: &'a [PathBuf],
    ) -> impl Iterator<Item = Cow<'a, OsStr>> {
        let flags = ["-l", "-F", "-S", "--"]
            .into_iter()
            .map(|f| Cow::Borrowed(OsStr::new(f)));

        flags
            .chain(std::iter::once(Cow::Borrowed(OsStr::new(query))))
            .chain(paths.iter().map(|p| Cow::Borrowed(p.as_os_str())))
    }
}
