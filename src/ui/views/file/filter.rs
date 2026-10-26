use std::{path::Path, sync::Arc};

use crate::utils::filekinds::{FileCategory, FileKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFilter {
    Dir,
    File,
    Category(FileCategory),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileFilter {
    pub types: Vec<TypeFilter>,
    pub exts: Vec<String>,
}

impl TypeFilter {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "d" | "dir" | "folder" => Self::Dir,
            "f" | "file" => Self::File,
            other => Self::Category(FileCategory::from_alias(other)?),
        })
    }

    fn matches(self, kind: FileKind) -> bool {
        match self {
            Self::Dir => kind.is_dir(),
            Self::File => !kind.is_dir(),
            Self::Category(c) => kind.category() == c,
        }
    }
}

impl FileFilter {
    pub fn parse(query: &Arc<str>) -> (Self, Arc<str>) {
        let mut filter = Self::default();

        let mut found_filter = false;
        let text: Vec<&str> = query
            .split_whitespace()
            .filter(|token| {
                let is_filter = filter.try_apply(token);
                found_filter |= is_filter;
                !is_filter
            })
            .collect();

        if found_filter {
            (filter, text.join(" ").into())
        } else {
            (filter, Arc::clone(query))
        }
    }

    fn try_apply(&mut self, token: &str) -> bool {
        let Some((key, value)) = token.split_once(':') else {
            return false;
        };

        if value.is_empty() {
            return false;
        }

        let value = value.to_ascii_lowercase();
        match key.to_ascii_lowercase().as_str() {
            "t" | "type" => {
                let parsed: Option<Vec<_>> = value.split(',').map(TypeFilter::parse).collect();
                parsed.map(|types| self.types.extend(types)).is_some()
            }
            "e" | "ext" => {
                self.exts.extend(
                    value
                        .split(',')
                        .map(|e| e.trim_start_matches('.'))
                        .filter(|e| !e.is_empty())
                        .map(String::from),
                );
                true
            }
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty() && self.exts.is_empty()
    }

    pub fn matches(&self, path: &Path, is_dir: bool, is_symlink: bool) -> bool {
        let ext = path.extension().and_then(|e| e.to_str());

        if !self.exts.is_empty() {
            let ok =
                !is_dir && ext.is_some_and(|x| self.exts.iter().any(|e| e.eq_ignore_ascii_case(x)));
            if !ok {
                return false;
            }
        }

        if !self.types.is_empty() {
            let kind = FileKind::from_extension(ext, is_symlink, is_dir);
            if !self.types.iter().any(|t| t.matches(kind)) {
                return false;
            }
        }

        true
    }

    pub fn dirs_only(&self) -> bool {
        !self.types.is_empty() && self.types.iter().all(|t| *t == TypeFilter::Dir)
    }

    /// Extensions a backend can pre-filter on, or None if the filter can't be
    /// expressed as extensions
    pub fn pushdown_exts(&self) -> Option<Vec<&str>> {
        if !self.exts.is_empty() {
            return Some(self.exts.iter().map(String::as_str).collect());
        }
        let mut out = Vec::new();
        for t in &self.types {
            match t {
                TypeFilter::Category(c) => {
                    let exts = c.extensions()?;
                    out.extend(exts)
                }
                _ => return None,
            }
        }
        (!out.is_empty()).then_some(out)
    }
}
