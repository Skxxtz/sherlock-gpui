use std::path::{Path, PathBuf};

use super::*;

/// Compares at the 8-bit precision themes are stored in.
fn same(a: &ThemeData, b: &ThemeData) -> bool {
    ThemeData::to_table(a).unwrap() == ThemeData::to_table(b).unwrap()
}

fn hex(c: Hsla) -> String {
    toml::Value::try_from(c).unwrap().to_string()
}

fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sherlock-theme-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, file: &str, content: &str) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn hex_keeps_alpha() {
    let c = parse_hex_color("#ffffff80").unwrap();
    assert!((c.a - 128.0 / 255.0).abs() < 1e-3);
    assert_eq!(parse_hex_color("#fff").unwrap().a, 1.0);
    assert_eq!(parse_hex_color("000000").unwrap().a, 1.0);
    assert!(parse_hex_color("#12345").is_none());
    assert!(parse_hex_color("#gggggg").is_none());
}

#[test]
fn builtin_round_trips_through_table() {
    let table = ThemeData::to_table(&ThemeData::nord()).unwrap();
    let back: ThemeData = toml::Value::Table(table).try_into().unwrap();
    assert!(same(&back, &ThemeData::nord()));
}

#[test]
fn partial_theme_extends_builtin() {
    let d = dir("partial");
    let path = write(
        &d,
        "mine.toml",
        "extends = \"catppuccin_mocha\"\nbg_app = \"#00000080\"\nradius = 12.0\n",
    );
    let theme = ThemeData::from_file(&path).unwrap();
    let base = ThemeData::catppuccin_mocha();
    assert_eq!(hex(theme.primary_text), hex(base.primary_text));
    assert!((theme.bg_app.a - 128.0 / 255.0).abs() < 1e-3);
    assert_eq!(theme.metrics.radius, 12.0);
    assert_eq!(theme.typography, base.typography);
}

#[test]
fn no_extends_uses_default() {
    let d = dir("default");
    let path = write(&d, "mine.toml", "font_size = 15.0\n");
    let theme = ThemeData::from_file(&path).unwrap();
    assert_eq!(theme.typography.font_size, 15.0);
    let mut expected = ThemeData::dark();
    expected.typography.font_size = 15.0;
    assert!(same(&theme, &expected));
}

#[test]
fn extends_chain_of_files() {
    let d = dir("chain");
    write(
        &d,
        "base.toml",
        "extends = \"nord\"\ncursor = \"#ff0000\"\n",
    );
    let path = write(&d, "child.toml", "extends = \"base\"\nradius = 2.0\n");
    let theme = ThemeData::from_file(&path).unwrap();
    assert_eq!(hex(theme.cursor), hex(parse_hex_color("#ff0000").unwrap()));
    assert_eq!(hex(theme.bg_app), hex(ThemeData::nord().bg_app));
    assert_eq!(theme.metrics.radius, 2.0);
}

#[test]
fn cycles_and_missing_bases_error() {
    let d = dir("errors");
    write(&d, "a.toml", "extends = \"b\"\n");
    let a = d.join("a.toml");
    write(&d, "b.toml", "extends = \"a\"\n");
    assert!(ThemeData::from_file(&a).unwrap_err().contains("circular"));

    let missing = write(&d, "c.toml", "extends = \"nope\"\n");
    assert!(ThemeData::from_file(&missing).unwrap_err().contains("nope"));

    let bad = write(&d, "d.toml", "bg_app = \"#zzz\"\n");
    assert!(
        ThemeData::from_file(&bad)
            .unwrap_err()
            .contains("invalid color")
    );
}
