use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
};

use super::{
    Location,
    utils::{file_err, is_json, read_or_empty},
};
use crate::utils::{
    config::ConfigSource,
    errors::{SherlockMessage, types::FileAction},
};

/// For each dotted key: the first file (depth-first through `[[source]]`) that contains
/// the key, else the first file that contains its section, else `start`.
pub fn locate_keys<'a>(
    keys: &[&str],
    start: &'a Path,
) -> Result<Vec<Location<'a>>, SherlockMessage> {
    let split: Vec<(Option<&str>, &str)> = keys
        .iter()
        .map(|k| match k.rsplit_once('.') {
            Some((s, k)) => (Some(s), k),
            None => (None, *k),
        })
        .collect();

    let mut out: Vec<Location<'a>> = keys
        .iter()
        .map(|_| Location {
            path: Cow::Borrowed(start),
            contains_key: false,
        })
        .collect();
    let mut section_seen = vec![false; keys.len()];
    let mut open = keys.len();

    let mut visited = HashSet::new();
    let mut stack = vec![start.to_path_buf()];

    while let Some(file) = stack.pop() {
        if open == 0 {
            break;
        }
        let canon = fs::canonicalize(&file).unwrap_or_else(|_| file.clone());
        if !visited.insert(canon) {
            continue;
        }

        let mut table = read_table(&file)?;
        let sources = take_sources(&mut table, &file)?;

        for (i, (sec, key)) in split.iter().enumerate() {
            if out[i].contains_key {
                continue;
            }
            let Some(t) = section(&table, *sec) else {
                continue;
            };
            let hit = t.contains_key(*key);
            if hit || !section_seen[i] {
                section_seen[i] = true;
                out[i] = Location {
                    path: as_cow(&file, start),
                    contains_key: hit,
                };
                if hit {
                    open -= 1;
                }
            }
        }

        stack.extend(sources.into_iter().rev());
    }

    Ok(out)
}

fn section<'t>(root: &'t toml::Table, path: Option<&str>) -> Option<&'t toml::Table> {
    match path {
        None => Some(root),
        Some(p) => p.split('.').try_fold(root, |t, s| t.get(s)?.as_table()),
    }
}

pub fn lookup<'t>(root: &'t toml::Table, segs: &[&str]) -> Option<&'t toml::Value> {
    let (last, init) = segs.split_last()?;
    let t = init.iter().try_fold(root, |t, s| t.get(*s)?.as_table())?;
    t.get(*last)
}

fn as_cow<'a>(file: &Path, start: &'a Path) -> Cow<'a, Path> {
    if file == start {
        Cow::Borrowed(start)
    } else {
        Cow::Owned(file.to_path_buf())
    }
}

/// Parses a TOML or JSON config into a table. A missing file is empty.
fn read_table(path: &Path) -> Result<toml::Table, SherlockMessage> {
    let content = read_or_empty(path)?;
    if content.trim().is_empty() {
        return Ok(toml::Table::new());
    }
    let parse = |e: &dyn Display| file_err(FileAction::Parse, path, e);
    if is_json(path) {
        serde_json::from_str(&content).map_err(|e| parse(&e))
    } else {
        content.parse::<toml::Table>().map_err(|e| parse(&e))
    }
}

/// Pulls `[[source]]` out of the parsed table, resolved relative to `path`.
fn take_sources(table: &mut toml::Table, path: &Path) -> Result<Vec<PathBuf>, SherlockMessage> {
    let Some(v) = table.remove("source") else {
        return Ok(Vec::new());
    };
    let sources: Vec<ConfigSource> = v
        .try_into()
        .map_err(|e| file_err(FileAction::Parse, path, &e))?;
    let base = path.parent().unwrap_or(Path::new("."));
    Ok(sources
        .into_iter()
        .map(|s| {
            if s.file.is_relative() {
                base.join(s.file)
            } else {
                s.file
            }
        })
        .collect())
}

/// All files reachable via `[[source]]` from `start` (excluding `start`).
pub fn config_sources(start: &Path) -> HashSet<PathBuf> {
    let mut seen = HashSet::new();
    let mut stack = vec![start.to_path_buf()];
    while let Some(file) = stack.pop() {
        let Ok(mut table) = read_table(&file) else {
            continue;
        };
        for src in take_sources(&mut table, &file).unwrap_or_default() {
            if seen.insert(src.clone()) {
                stack.push(src);
            }
        }
    }
    seen
}
