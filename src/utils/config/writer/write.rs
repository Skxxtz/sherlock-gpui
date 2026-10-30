use std::{fmt::Display, fs, path::Path};

use toml_edit::{DocumentMut, Item, TableLike};

use super::{
    locate::lookup,
    utils::{file_err, is_json, read_or_empty, ser_err},
};
use crate::utils::errors::{SherlockMessage, types::FileAction};

/// Writes `keys` (dotted paths) with their values from `new` into `path`, keeping
/// comments and formatting. Keys missing from `new` are removed.
pub fn write_keys(path: &Path, keys: &[&str], new: &toml::Table) -> Result<(), SherlockMessage> {
    let current = read_or_empty(path)?;
    let parse = |e: &dyn Display| file_err(FileAction::Parse, path, e);

    let out = if is_json(path) {
        let mut file: serde_json::Value = if current.trim().is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(&current).map_err(|e| parse(&e))?
        };
        for key in keys {
            let segs: Vec<&str> = key.split('.').collect();
            set_json(&mut file, &segs, lookup(new, &segs));
        }
        serde_json::to_string_pretty(&file).map_err(ser_err)?
    } else {
        let mut doc: DocumentMut = current.parse().map_err(|e| parse(&e))?;
        for key in keys {
            let segs: Vec<&str> = key.split('.').collect();
            set_toml(doc.as_table_mut(), &segs, lookup(new, &segs), false);
        }
        doc.to_string()
    };

    if out == current {
        return Ok(());
    }
    fs::write(path, out).map_err(|e| file_err(FileAction::Write, path, &e))
}

pub fn set_toml(doc: &mut dyn TableLike, segs: &[&str], val: Option<&toml::Value>, inline: bool) {
    match segs {
        [] => {}
        [key] => match val {
            Some(nv) => match doc.get_mut(key).and_then(Item::as_value_mut) {
                // keeps the comment / spacing around the old value
                Some(v) => {
                    let decor = v.decor().clone();
                    *v = to_value(nv);
                    *v.decor_mut() = decor;
                }
                None => {
                    doc.insert(key, Item::Value(to_value(nv)));
                }
            },
            None => {
                doc.remove(key);
            }
        },
        [key, rest @ ..] => {
            if doc.get(key).and_then(Item::as_table_like).is_none() {
                if val.is_none() {
                    return; // nothing to remove
                }
                doc.insert(key, new_table(inline));
            }
            if let Some(item) = doc.get_mut(key) {
                let child_inline = item.is_inline_table();
                if let Some(t) = item.as_table_like_mut() {
                    set_toml(t, rest, val, child_inline);
                }
            }
        }
    }
}

pub fn set_json(file: &mut serde_json::Value, segs: &[&str], val: Option<&toml::Value>) {
    use serde_json::Value;
    if !file.is_object() {
        *file = Value::Object(Default::default());
    }
    let Value::Object(f) = file else { return };
    match segs {
        [] => {}
        [key] => match val {
            Some(v) => {
                f.insert(
                    key.to_string(),
                    serde_json::to_value(v).unwrap_or(Value::Null),
                );
            }
            None => {
                f.remove(*key);
            }
        },
        [key, rest @ ..] => {
            if val.is_none() && !f.contains_key(*key) {
                return;
            }
            set_json(f.entry(key.to_string()).or_insert(Value::Null), rest, val);
        }
    }
}

/// An empty table that only shows up in the file once it holds a value.
pub fn new_table(inline: bool) -> Item {
    if inline {
        Item::Value(toml_edit::Value::InlineTable(Default::default()))
    } else {
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        Item::Table(t)
    }
}

pub fn to_value(v: &toml::Value) -> toml_edit::Value {
    match v {
        toml::Value::String(s) => s.as_str().into(),
        toml::Value::Integer(i) => (*i).into(),
        toml::Value::Float(f) => (*f).into(),
        toml::Value::Boolean(b) => (*b).into(),
        toml::Value::Datetime(d) => d
            .to_string()
            .parse::<toml_edit::Datetime>()
            .map(Into::into)
            .unwrap_or_else(|_| d.to_string().into()),
        toml::Value::Array(a) => a.iter().map(to_value).collect::<toml_edit::Array>().into(),
        toml::Value::Table(t) => t
            .iter()
            .map(|(k, v)| (k.clone(), to_value(v)))
            .collect::<toml_edit::InlineTable>()
            .into(),
    }
}
