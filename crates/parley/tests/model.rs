use parley::model::Model;
use parley::{Intent, Lexicon, SlotKind, parse, parse_with};

#[test]
fn embedded_model_loads_and_parses() {
    let p = parse("new 20m timer then notify me").expect("model should be embedded");
    assert_eq!(p.intent, Intent::Timer);
    assert_eq!(p.slot(SlotKind::Duration), Some("20m"));
    assert_eq!(p.slot(SlotKind::Action), Some("notify me"));
}

#[test]
fn keeps_original_spelling() {
    let p = parse("Remind me in 10 Minutes to Call Mom").unwrap();
    assert_eq!(p.intent, Intent::Remind);
    assert_eq!(p.slot(SlotKind::Duration), Some("10 Minutes"));
    assert_eq!(p.slot(SlotKind::Text), Some("Call Mom"));
}

#[test]
fn lexicon_helps_with_unknown_apps() {
    let lex = Lexicon::with_apps(["Zed", "Bitwarden"]);
    let p = parse_with("launch bitwarden", &lex).unwrap();
    assert_eq!(p.intent, Intent::Open);
    assert_eq!(p.slot(SlotKind::App), Some("bitwarden"));
}

#[test]
fn non_commands_are_none() {
    for q in ["", "   ", "firefox", "play some music", "2 + 3"] {
        assert_eq!(parse(q), None, "{q:?}");
    }
}

#[test]
fn slots_respect_the_intent() {
    for q in [
        "new 20m timer then notify me",
        "volume 30%",
        "open kitty",
        "search gpui docs",
    ] {
        let p = parse(q).unwrap();
        for s in &p.slots {
            assert!(p.intent.slots().contains(&s.kind), "{q}: {s:?}");
        }
    }
}

#[test]
fn file_round_trip_and_rejects_garbage() {
    let m = Model::new();
    assert_eq!(Model::from_bytes(&m.to_bytes()), Ok(m));
    assert!(Model::from_bytes(b"").is_err());
    assert!(Model::from_bytes(b"NOPE").is_err());
    let mut b = Model::new().to_bytes();
    b[6] ^= 1; // corrupt the schema length
    assert!(Model::from_bytes(&b).is_err());
}
