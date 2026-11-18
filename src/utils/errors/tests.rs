use std::collections::HashSet;

use super::*;

fn read(path: &str) -> Result<String, SherlockMessage> {
    Ok(std::fs::read_to_string(path)?)
}

#[test]
fn question_mark_converts_and_locates() {
    let e = read("/definitely/not/here").unwrap_err();
    assert_eq!(e.kind, ErrorKind::Io);
    assert_eq!(e.location, "src/utils/errors/tests.rs:6"); // the `?` line
    assert!(!e.cause.is_empty());
}

#[test]
fn or_kind_locates_the_caller() {
    let line = line!() + 2; // the `.or_kind` line
    let e = Err::<(), _>("boom")
        .or_kind(ErrorKind::InvalidData)
        .unwrap_err();
    assert_eq!(e.location, format!("src/utils/errors/tests.rs:{line}"));
    assert_eq!(e.level, SherlockMessageLevel::Warning);
    assert_eq!(e.to_string(), "Invalid data: boom");
}

#[test]
fn level_comes_from_the_kind() {
    let bug = sherlock_msg!(ErrorKind::bug("x"), "");
    assert_eq!(bug.level, SherlockMessageLevel::Error);
    let demo = Err::<(), _>("x").or_kind(ErrorKind::Demo).unwrap_err();
    assert_eq!(demo.level, SherlockMessageLevel::Info);
    // explicit level still wins
    let e = sherlock_msg!(Info, ErrorKind::bug("x"), "");
    assert_eq!(e.level, SherlockMessageLevel::Info);
}

#[test]
fn context_chains_outermost_first() {
    let e = read("/nope")
        .or_kind_with(|| ErrorKind::fs(FsOp::Read, "/nope"))
        .context("reading aliases")
        .context("loading launchers")
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::fs(FsOp::Read, "/nope"));
    // the inner Io kind moved into the context
    assert_eq!(
        e.context,
        vec![
            "loading launchers".into(),
            "reading aliases".into(),
            SharedString::from("I/O error")
        ]
    );
    assert!(
        e.details()
            .starts_with("loading launchers\n  reading aliases\n    I/O error\n")
    );
}

#[test]
fn option_and_levels() {
    let e = None::<u8>
        .or_kind(ErrorKind::env("TERMINAL"))
        .unwrap_err()
        .error_level();
    assert_eq!(e.level, SherlockMessageLevel::Error);
    assert_eq!(e.to_string(), "$TERMINAL is not set");
    assert_eq!(e.kind.name(), "Env");
    assert_eq!(e.kind.code(), "S0202");
}

#[test]
fn json_file_errors_carry_position_and_remedy() {
    let text = "{\n  \"a\": 1\n  \"b\": 2\n}";
    let e = serde_json::from_str::<serde_json::Value>(text)
        .in_file("/cfg/fallback.json")
        .unwrap_err();
    let pos = Pos { line: 3, col: 3 };
    assert_eq!(
        e.kind,
        ErrorKind::parse_file(Format::Json, "/cfg/fallback.json", Some(pos))
    );
    assert_eq!(
        e.kind.to_string(),
        "Failed to parse json file \"/cfg/fallback.json\" at 3:3"
    );
    assert_eq!(
        e.kind.remedy(),
        Some(Remedy::OpenFile("/cfg/fallback.json".into(), Some(pos)))
    );
}

#[test]
fn toml_and_simd_positions_need_the_text() {
    let text = "a = 1\nb = = 2\n";
    let e = text
        .parse::<toml::Table>()
        .in_file_text("/c.toml", text)
        .unwrap_err();
    let ErrorKind::ParseFile { pos: Some(pos), .. } = e.kind else {
        panic!("{e:?}")
    };
    assert_eq!(pos.line, 2);

    let text = "[1,\n 2,,]";
    let mut bytes = text.as_bytes().to_vec();
    let e = simd_json::from_slice::<Vec<u8>>(&mut bytes)
        .in_file_text("/x.json", text)
        .unwrap_err();
    let ErrorKind::ParseFile { pos: Some(pos), .. } = e.kind else {
        panic!("{e:?}")
    };
    assert_eq!(pos.line, 2);
}

#[test]
fn report_has_code_and_hint() {
    let e = sherlock_msg!(ErrorKind::config("bad key"), "line 3");
    let report = e.to_report();
    assert!(
        report.starts_with("[Warning] S0201 Config error: bad key\n"),
        "{report}"
    );
    assert!(
        report.contains("hint: check your Sherlock config"),
        "{report}"
    );
}

#[test]
fn fingerprint_ignores_cause() {
    let make = |cause| sherlock_msg!(ErrorKind::Closed, cause);
    let (a, b) = (make("x"), make("y"));
    assert_ne!(a, b);
    assert_eq!(a.fingerprint(), b.fingerprint()); // same code, same line
    let other = sherlock_msg!(ErrorKind::Closed, "x");
    assert_ne!(a.fingerprint(), other.fingerprint()); // another place
}

#[test]
fn catalog_codes_are_unique_and_explained() {
    let mut seen = HashSet::new();
    for k in ErrorKind::CATALOG {
        assert!(seen.insert(k.code), "duplicate code {}", k.code);
        assert!(!k.doc.trim().is_empty(), "{} has no docs", k.code);
        assert_eq!(
            ErrorKind::explain(&k.code.to_lowercase()).unwrap().name,
            k.name
        );
    }
}

#[test]
fn macro_and_serde_roundtrip() {
    let kinds = [
        ErrorKind::config("bad key"),
        ErrorKind::parse_file(Format::Toml, "/a.toml", Some(Pos { line: 1, col: 2 })),
        ErrorKind::fs(FsOp::CreateDir, "/tmp/x"),
        ErrorKind::http(HttpMethod::Get, "wttr.in"),
        ErrorKind::Closed,
    ];
    for kind in kinds {
        let e = sherlock_msg!(Error, kind, "line 3");
        let json = serde_json::to_string(&e).unwrap();
        let back: SherlockMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
        assert!(back.logged); // the sender logs it
    }
}
