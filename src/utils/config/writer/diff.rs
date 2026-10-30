/// Dotted paths of every leaf key that was changed, added or removed.
pub fn changed_keys(old: &toml::Table, new: &toml::Table) -> Vec<String> {
    let mut out = Vec::new();
    diff_keys(old, new, &mut String::new(), &mut out);
    out
}

fn diff_keys(old: &toml::Table, new: &toml::Table, prefix: &mut String, out: &mut Vec<String>) {
    let empty = toml::Table::new();
    for (key, nv) in new {
        let ov = old.get(key);
        if ov == Some(nv) {
            continue;
        }
        let len = enter(prefix, key);
        match (ov, nv) {
            (Some(toml::Value::Table(ot)), toml::Value::Table(nt)) => {
                diff_keys(ot, nt, prefix, out)
            }
            (None, toml::Value::Table(nt)) => diff_keys(&empty, nt, prefix, out),
            _ => out.push(prefix.clone()),
        }
        prefix.truncate(len);
    }
    for key in old.keys().filter(|k| !new.contains_key(*k)) {
        let len = enter(prefix, key);
        out.push(prefix.clone());
        prefix.truncate(len);
    }
}

fn enter(prefix: &mut String, key: &str) -> usize {
    let len = prefix.len();
    if len > 0 {
        prefix.push('.');
    }
    prefix.push_str(key);
    len
}
