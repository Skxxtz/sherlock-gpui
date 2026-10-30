use std::{borrow::Cow, collections::HashMap, path::Path};

use crate::utils::{config::SherlockConfig, errors::SherlockMessage};

mod diff;
mod locate;
mod utils;
mod write;

#[cfg(test)]
mod tests;

use diff::changed_keys;
use locate::locate_keys;
use utils::ser_err;
use write::write_keys;

struct Location<'a> {
    pub path: Cow<'a, Path>,
    pub contains_key: bool,
}

/// Writes every key that changed from `before` to `after` into the file that defines it,
/// following `[[source]]` files. Keys not found anywhere go into `config`.
pub fn save_changes(
    config: &Path,
    before: &SherlockConfig,
    after: &SherlockConfig,
) -> Result<(), SherlockMessage> {
    let old = toml::Table::try_from(before).map_err(ser_err)?;
    let new = toml::Table::try_from(after).map_err(ser_err)?;

    let keys = changed_keys(&old, &new);
    if keys.is_empty() {
        return Ok(());
    }
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let locations = locate_keys(&refs, config)?;

    let mut per_file: HashMap<Cow<'_, Path>, Vec<&str>> = HashMap::new();
    for (key, loc) in refs.iter().zip(locations) {
        per_file.entry(loc.path).or_default().push(*key);
    }
    for (path, keys) in per_file {
        write_keys(&path, &keys, &new)?;
    }
    Ok(())
}
