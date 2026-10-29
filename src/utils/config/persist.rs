use std::{fs, path::Path};

use toml_edit::{DocumentMut, Item, TableLike};

use crate::{
    sherlock_msg,
    utils::{
        config::SherlockConfig,
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
    },
};

/// Writes the keys that changed from `before` to `after` into the config file at `path`
/// (TOML, or JSON by extension). A missing file is created with just those keys.
pub fn write_changes(
    path: &Path,
    before: &SherlockConfig,
    after: &SherlockConfig,
) -> Result<(), SherlockMessage> {
    let ser = |e: &dyn std::fmt::Display| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::SerializationError,
            e.to_string()
        )
    };
    let parse = |e: &dyn std::fmt::Display| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Parse, path.to_path_buf()),
            e.to_string()
        )
    };
    let current = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            ));
        }
    };

    let out = if path.extension().is_some_and(|e| e == "json") {
        let old = serde_json::to_value(before).map_err(|e| ser(&e))?;
        let new = serde_json::to_value(after).map_err(|e| ser(&e))?;
        let mut file: serde_json::Value = if current.trim().is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(&current).map_err(|e| parse(&e))?
        };
        patch_json(&mut file, &old, &new);
        serde_json::to_string_pretty(&file).map_err(|e| ser(&e))?
    } else {
        let old = toml::Table::try_from(before).map_err(|e| ser(&e))?;
        let new = toml::Table::try_from(after).map_err(|e| ser(&e))?;
        let mut doc: DocumentMut = current.parse().map_err(|e| parse(&e))?;
        patch_toml(doc.as_table_mut(), &old, &new, false);
        doc.to_string()
    };

    if out == current {
        return Ok(());
    }
    fs::write(path, out).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Write, path.to_path_buf()),
            e
        )
    })
}

/// Applies the differences between `old` and `new` to `doc`, recursing into tables.
fn patch_toml(doc: &mut dyn TableLike, old: &toml::Table, new: &toml::Table, inline: bool) {
    for (key, nv) in new {
        let ov = old.get(key);
        if ov == Some(nv) {
            continue;
        }
        if let toml::Value::Table(nt) = nv {
            let empty = toml::Table::new();
            let ot = match ov {
                Some(toml::Value::Table(t)) => t,
                _ => &empty,
            };
            if doc.get(key).and_then(Item::as_table_like).is_none() {
                doc.insert(key, new_table(inline));
            }
            if let Some(item) = doc.get_mut(key) {
                let child_inline = item.is_inline_table();
                if let Some(t) = item.as_table_like_mut() {
                    patch_toml(t, ot, nt, child_inline);
                }
            }
        } else {
            match doc.get_mut(key).and_then(Item::as_value_mut) {
                // keeps the comment / spacing around the old value
                Some(v) => {
                    let decor = v.decor().clone();
                    *v = to_value(nv);
                    *v.decor_mut() = decor;
                }
                None => {
                    doc.insert(key, Item::Value(to_value(nv)));
                }
            }
        }
    }
    for key in old.keys().filter(|k| !new.contains_key(*k)) {
        doc.remove(key);
    }
}

/// An empty table that only shows up in the file once it holds a value.
fn new_table(inline: bool) -> Item {
    if inline {
        Item::Value(toml_edit::Value::InlineTable(Default::default()))
    } else {
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        Item::Table(t)
    }
}

fn to_value(v: &toml::Value) -> toml_edit::Value {
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

/// [`patch_toml`] for JSON.
fn patch_json(file: &mut serde_json::Value, old: &serde_json::Value, new: &serde_json::Value) {
    use serde_json::Value;
    let (Value::Object(o), Value::Object(n)) = (old, new) else {
        if old != new {
            *file = new.clone();
        }
        return;
    };
    if !file.is_object() {
        *file = Value::Object(Default::default());
    }
    let Value::Object(f) = file else { return };
    for (key, nv) in n {
        match o.get(key) {
            Some(ov) if ov == nv => {}
            Some(ov @ Value::Object(_)) if nv.is_object() => {
                patch_json(f.entry(key.clone()).or_insert(Value::Null), ov, nv);
            }
            None if nv.is_object() => {
                let empty = Value::Object(Default::default());
                patch_json(f.entry(key.clone()).or_insert(Value::Null), &empty, nv);
            }
            _ => {
                f.insert(key.clone(), nv.clone());
            }
        }
    }
    for key in o.keys().filter(|k| !n.contains_key(*k)) {
        f.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toml_patch(file: &str, old: &str, new: &str) -> String {
        let mut doc: DocumentMut = file.parse().unwrap();
        let (old, new): (toml::Table, toml::Table) =
            (toml::from_str(old).unwrap(), toml::from_str(new).unwrap());
        patch_toml(doc.as_table_mut(), &old, &new, false);
        doc.to_string()
    }

    #[test]
    fn only_changed_keys_are_written_and_comments_stay() {
        let file = "# mine\n[behavior]\nanimate = true # keep\n";
        let old = "[behavior]\nanimate = true\nx = 1\n[units]\nlen = \"m\"\n";
        let new = "[behavior]\nanimate = false\nx = 1\n[units]\nlen = \"m\"\n";
        assert_eq!(
            toml_patch(file, old, new),
            "# mine\n[behavior]\nanimate = false # keep\n"
        );
    }

    #[test]
    fn missing_tables_are_created_with_just_the_change() {
        let old = "[appearance]\nwidth = 900\nheight = 600\n";
        let new = "[appearance]\nwidth = 1000\nheight = 600\n";
        assert_eq!(toml_patch("", old, new), "[appearance]\nwidth = 1000\n");
    }

    #[test]
    fn inline_tables_are_patched_in_place() {
        let file = "units = { len = \"m\" }\n";
        let old = "[units]\nlen = \"m\"\n";
        let new = "[units]\nlen = \"km\"\n";
        assert_eq!(toml_patch(file, old, new), "units = { len = \"km\" }\n");
    }

    #[test]
    fn removed_keys_are_removed() {
        let file = "[a]\nb = 1\nc = 2\n";
        assert_eq!(
            toml_patch(file, "[a]\nb = 1\nc = 2\n", "[a]\nb = 1\n"),
            "[a]\nb = 1\n"
        );
    }

    #[test]
    fn json_only_changed_keys() {
        use serde_json::json;
        let mut file = json!({ "behavior": { "animate": true } });
        let old = json!({ "behavior": { "animate": true, "x": 1 }, "units": { "len": "m" } });
        let new = json!({ "behavior": { "animate": false, "x": 1 }, "units": { "len": "km" } });
        patch_json(&mut file, &old, &new);
        assert_eq!(
            file,
            json!({ "behavior": { "animate": false }, "units": { "len": "km" } })
        );
    }
}
