use gpui::Keystroke;

use super::state::*;

fn state(chords: &[&'static str]) -> KeymapState {
    KeymapState::new(chords.iter().map(|&c| c.into()).collect())
}

fn key(s: &str) -> Keystroke {
    Keystroke::parse(s).unwrap()
}

#[test]
fn adds_replaces_and_removes() {
    let mut s = state(&["ctrl-n", "down"]);
    s.record_at(s.add_line());
    assert!(s.record(&key("ctrl-j")));
    assert_eq!(s.chords, ["ctrl-n", "down", "ctrl-j"]);
    assert_eq!(s.cursor, 2, "cursor on the new chord");

    s.record_at(0);
    assert!(s.record(&key("alt-n")));
    assert_eq!(s.chords, ["alt-n", "down", "ctrl-j"]);

    s.record_at(1);
    assert!(s.record(&key("backspace")));
    assert_eq!(s.chords, ["alt-n", "ctrl-j"]);
    assert!(s.recording.is_none());
}

#[test]
fn modifiers_wait_escape_cancels_duplicates_jump() {
    let mut s = state(&["ctrl-n"]);
    s.record_at(s.add_line());
    assert!(!s.record(&key("control")));
    assert!(
        s.recording.is_some(),
        "a lone modifier doesn't finish the chord"
    );
    assert!(!s.record(&key("escape")));
    assert!(s.recording.is_none());

    s.record_at(s.add_line());
    assert!(
        !s.record(&key("ctrl-n")),
        "an existing chord isn't added twice"
    );
    assert_eq!(s.cursor, 0);
    assert_eq!(s.chords, ["ctrl-n"]);
}

#[test]
fn cursor_stays_in_range() {
    let mut s = state(&["a", "b"]);
    s.set_cursor(9);
    assert_eq!(s.cursor, 2);
    s.remove(1);
    assert_eq!(s.cursor, 1);
    assert!(!s.remove(5));
}
