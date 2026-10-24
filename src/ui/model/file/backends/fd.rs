use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    ui::model::file::{
        backends::command::CommandFactory,
        filter::{FileFilter, TypeFilter},
    },
    utils::filekinds::FileCategory,
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FdFactory;

impl FdFactory {
    /// fd type letters for the filter: f = file, d = directory, l = symlink.
    fn types(filter: &FileFilter) -> Vec<&'static str> {
        // Extensions imply regular files.
        if !filter.exts.is_empty() {
            return vec!["f"];
        }
        if filter.types.is_empty() {
            return vec!["f", "d"];
        }
        let mut out = Vec::new();
        for t in &filter.types {
            let ty = match t {
                TypeFilter::Dir | TypeFilter::Category(FileCategory::Directory) => "d",
                TypeFilter::Category(FileCategory::Link) => "l",
                _ => "f",
            };
            if !out.contains(&ty) {
                out.push(ty);
            }
        }
        out
    }
}

impl CommandFactory for FdFactory {
    const BINARY_NAME: &'static str = "fd";
    const HANDLES_FILTERING: bool = false;

    fn args<'a>(
        &self,
        _query: &'a str,
        filter: &'a FileFilter,
        paths: &'a [PathBuf],
    ) -> impl Iterator<Item = Cow<'a, OsStr>> {
        let base = [".", "--color", "never"].into_iter();

        let types = Self::types(filter).into_iter().flat_map(|t| ["--type", t]);

        let exts = filter
            .pushdown_exts()
            .into_iter()
            .flatten()
            .flat_map(|e| ["--extension", e]);

        base.chain(types)
            .chain(exts)
            .map(|s| Cow::Borrowed(OsStr::new(s)))
            .chain(paths.iter().flat_map(|p| {
                [
                    Cow::Borrowed(OsStr::new("--search-path")),
                    Cow::Borrowed(p.as_os_str()),
                ]
            }))
    }
}
