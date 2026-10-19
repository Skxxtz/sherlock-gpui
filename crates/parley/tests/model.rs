use parley::model::Model;
use parley::{Intent, Lexicon, SlotKind, check_model, parse, parse_with};

#[test]
fn embedded_model_is_valid() {
    if let Err(e) = check_model() {
        panic!("embedded model unusable: {e}");
    }
}

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
    assert!(Model::from_bytes(b"").unwrap_err().contains("parley-train"));
    assert!(Model::from_bytes(b"NOPE").is_err());
    let mut b = Model::new().to_bytes();
    b[6] ^= 1; // corrupt the schema length
    assert!(Model::from_bytes(&b).is_err());
}

#[test]
fn cleanup_rules() {
    use parley::model::trim_connectors;
    use parley::{SlotKind::*, Tag::*};
    let w = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();

    // leading connector leaves the slot
    let mut t = vec![O, B(Action), I(Action)];
    trim_connectors(&w("timer then ring"), &mut t);
    assert_eq!(t, vec![O, O, B(Action)]);

    // adjacent same-kind slots merge
    let mut t = vec![O, B(Percent), B(Percent)];
    trim_connectors(&w("volume 30 %"), &mut t);
    assert_eq!(t, vec![O, B(Percent), I(Percent)]);

    // trailing filler leaves the slot
    let mut t = vec![B(Action), I(Action)];
    trim_connectors(&w("ring please"), &mut t);
    assert_eq!(t, vec![B(Action), O]);
}

#[test]
fn recognisers() {
    use parley::SlotKind::*;
    use parley::features::recognised;
    let r = |q: &str| recognised(q);
    assert_eq!(r("timer 20m"), vec![(Duration, "20m".into())]);
    assert_eq!(r("in 1h 30m"), vec![(Duration, "1h 30m".into())]);
    assert_eq!(
        r("in half an hour"),
        vec![(Duration, "half an hour".into())]
    );
    assert_eq!(
        r("a couple of minutes"),
        vec![(Duration, "a couple of minutes".into())]
    );
    assert_eq!(
        r("2 and a half hours"),
        vec![(Duration, "2 and a half hours".into())]
    );
    assert_eq!(
        r("twenty minutes"),
        vec![(Duration, "twenty minutes".into())]
    );
    assert_eq!(r("at 5 pm tomorrow"), vec![(Time, "5 pm tomorrow".into())]);
    assert_eq!(r("quarter to 7"), vec![(Time, "quarter to 7".into())]);
    assert_eq!(r("volume 30 %"), vec![(Percent, "30 %".into())]);
    assert_eq!(r("volume max"), vec![(Percent, "max".into())]);
    assert_eq!(r("set a timer"), vec![]);
    assert_eq!(r("open firefox"), vec![]);
}
