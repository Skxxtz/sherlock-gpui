use std::path::PathBuf;

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
    let line = line!() + 1;
    let e = Err::<(), _>("boom")
        .or_kind(ErrorKind::InvalidData)
        .unwrap_err();
    assert_eq!(e.location, format!("src/utils/errors/tests.rs:{line}"));
    assert_eq!(e.level, SherlockMessageLevel::Warning);
    assert_eq!(e.to_string(), "Invalid data: boom");
}

#[test]
fn context_chains_outermost_first() {
    let e = read("/nope")
        .or_kind_with(|| ErrorKind::File(FileAction::Read, PathBuf::from("/nope")))
        .context("reading aliases")
        .context("loading launchers")
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::File(FileAction::Read, "/nope".into()));
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
        .or_kind(ErrorKind::Env("TERMINAL".into()))
        .unwrap_err()
        .as_error();
    assert_eq!(e.level, SherlockMessageLevel::Error);
    assert_eq!(e.to_string(), "Failed to read \"TERMINAL\"");
    assert_eq!(e.kind.name(), "Env");
}

#[test]
fn macro_and_serde_roundtrip() {
    let e = sherlock_msg!(Error, ErrorKind::Config("bad key".into()), "line 3");
    let json = serde_json::to_string(&e).unwrap();
    let back: SherlockMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(e, back);
    assert!(back.logged); // the sender logs it
}
