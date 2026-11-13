use toml_edit::DocumentMut;

use super::{
    diff::changed_keys,
    locate::lookup,
    write::{set_json, set_toml},
};

fn apply(file: &str, new: &str, keys: &[&str]) -> String {
    let mut doc: DocumentMut = file.parse().unwrap();
    let new: toml::Table = toml::from_str(new).unwrap();
    for key in keys {
        let segs: Vec<&str> = key.split('.').collect();
        set_toml(doc.as_table_mut(), &segs, lookup(&new, &segs), false);
    }
    doc.to_string()
}

#[test]
fn diff_finds_changed_added_and_removed_leaves() {
    let old: toml::Table = toml::from_str("[a]\nb = 1\nc = 2\n[d]\ne = 1\n").unwrap();
    let new: toml::Table = toml::from_str("[a]\nb = 2\n[d]\ne = 1\n[f]\ng = 1\n").unwrap();
    let mut keys = changed_keys(&old, &new);
    keys.sort();
    assert_eq!(keys, ["a.b", "a.c", "f.g"]);
}

#[test]
fn only_given_keys_are_written_and_comments_stay() {
    let file = "# mine\n[behavior]\nanimate = true # keep\n";
    let new = "[behavior]\nanimate = false\nx = 1\n";
    assert_eq!(
        apply(file, new, &["behavior.animate"]),
        "# mine\n[behavior]\nanimate = false # keep\n"
    );
}

#[test]
fn missing_tables_are_created_with_just_the_key() {
    let new = "[appearance]\nwidth = 1000\nheight = 600\n";
    assert_eq!(
        apply("", new, &["appearance.width"]),
        "[appearance]\nwidth = 1000\n"
    );
}

#[test]
fn inline_tables_are_patched_in_place() {
    let file = "units = { len = \"m\" }\n";
    assert_eq!(
        apply(file, "[units]\nlen = \"km\"\n", &["units.len"]),
        "units = { len = \"km\" }\n"
    );
}

#[test]
fn removed_keys_are_removed() {
    let file = "[a]\nb = 1\nc = 2\n";
    assert_eq!(apply(file, "[a]\nb = 1\n", &["a.c"]), "[a]\nb = 1\n");
}

#[test]
fn json_keys_are_written() {
    use serde_json::json;
    let mut file = json!({ "behavior": { "animate": true } });
    let new: toml::Table = toml::from_str("[behavior]\nanimate = false\n").unwrap();
    set_json(
        &mut file,
        &["behavior", "animate"],
        lookup(&new, &["behavior", "animate"]),
    );
    assert_eq!(file, json!({ "behavior": { "animate": false } }));
}
