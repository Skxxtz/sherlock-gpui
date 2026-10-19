#![cfg(feature = "training")]

use std::collections::HashSet;

use parley::training::Rng;
use parley::training::corpus::{Dataset, Split};
use parley::{Intent, Tag};

fn ds() -> Dataset {
    Dataset::bundled().unwrap()
}

#[test]
fn bundled_files_parse() {
    let d = ds();
    assert!(d.template_count(Split::Train) > 0);
    assert!(d.template_count(Split::Dev) > 0);
    assert!(d.template_count(Split::Test) > 0);
}

#[test]
fn every_intent_is_in_every_split() {
    let d = ds();
    let mut rng = Rng::new(1);
    for split in Split::ALL {
        let seen: HashSet<_> = d
            .generate(split, split, 5000, &mut rng, false)
            .iter()
            .map(|e| e.intent)
            .collect();
        for i in Intent::ALL {
            assert!(seen.contains(&Some(*i)), "{i:?} missing in {split:?}");
        }
        assert!(seen.contains(&None), "no `none` examples in {split:?}");
    }
}

#[test]
fn tags_are_consistent() {
    let mut rng = Rng::new(2);
    for ex in ds().generate(Split::Train, Split::Train, 5000, &mut rng, true) {
        assert_eq!(ex.words.len(), ex.tags.len());
        for (i, t) in ex.tags.iter().enumerate() {
            // I-X must follow B-X or I-X
            if let Tag::I(k) = t {
                assert!(
                    matches!(ex.tags[i - 1], Tag::B(p) | Tag::I(p) if p == *k),
                    "{ex:?}"
                );
            }
            // slots only where the intent allows them
            if let Tag::B(k) = t {
                assert!(ex.intent.unwrap().slots().contains(k), "{ex:?}");
            }
        }
    }
}

#[test]
fn multi_word_optional_groups() {
    let d = Dataset::parse(
        "timer: timer {DUR} [then {ACTION}]",
        "DUR = 5m\nDUR? = 6m\nDUR! = 7m\nACTION = beep\nACTION? = ring\nACTION! = buzz",
    )
    .unwrap();
    let mut rng = Rng::new(4);
    let texts: HashSet<String> = d
        .generate(Split::Train, Split::Train, 200, &mut rng, false)
        .iter()
        .map(|e| e.text())
        .collect();
    assert_eq!(
        texts,
        HashSet::from(["timer 5m".to_string(), "timer 5m then beep".to_string()])
    );
}

#[test]
fn intents_are_balanced() {
    let mut rng = Rng::new(5);
    let ex = ds().generate(Split::Train, Split::Train, 12_000, &mut rng, false);
    let n = Intent::ALL.len() + 1; // + none
    for i in Intent::ALL.iter().map(|i| Some(*i)).chain([None]) {
        let share = ex.iter().filter(|e| e.intent == i).count() as f64 / ex.len() as f64;
        assert!((share - 1.0 / n as f64).abs() < 0.03, "{i:?}: {share:.3}");
    }
}

#[test]
fn templates_may_put_punctuation_next_to_slots() {
    let d = Dataset::parse(
        "remind: {TEXT}, at {TIME}\nremind: reminder: {TEXT} {TIME}.",
        "TEXT = call mom\nTEXT? = a\nTEXT! = b\nTIME = 5pm\nTIME? = 6pm\nTIME! = 7pm",
    )
    .unwrap();
    let mut rng = Rng::new(9);
    let texts: HashSet<String> = d
        .generate(Split::Train, Split::Train, 100, &mut rng, false)
        .iter()
        .map(|e| e.text())
        .collect();
    assert_eq!(
        texts,
        HashSet::from([
            "call mom , at 5pm".to_string(),
            "reminder : call mom 5pm .".to_string()
        ])
    );
}

#[test]
fn same_seed_same_data() {
    let d = ds();
    let a = d.generate(Split::Train, Split::Train, 100, &mut Rng::new(7), true);
    let b = d.generate(Split::Train, Split::Train, 100, &mut Rng::new(7), true);
    assert_eq!(a, b);
}

fn skeletons(d: &Dataset, split: Split, n: usize, seed: u64) -> HashSet<String> {
    let mut rng = Rng::new(seed);
    d.generate(split, split, n, &mut rng, false)
        .iter()
        .map(|e| e.skeleton())
        .collect()
}

#[test]
fn held_out_phrasings_never_appear_in_training() {
    let d = ds();
    let train = skeletons(&d, Split::Train, 80_000, 3);
    for split in [Split::Dev, Split::Test] {
        for s in skeletons(&d, split, 4000, 4) {
            assert!(
                !train.contains(&s),
                "leak: {split:?} phrasing `{s}` is also a training phrasing"
            );
        }
    }
}

#[test]
fn held_out_values_are_new() {
    use parley::SlotKind;
    let d = ds();
    for kind in SlotKind::ALL {
        let train = d.values_in(Split::Train, *kind);
        let dev = d.values_in(Split::Dev, *kind);
        for v in d.values_in(Split::Dev, *kind) {
            assert!(
                !train.contains(&v),
                "{kind:?} dev value `{v}` is also a training value"
            );
        }
        for v in d.values_in(Split::Test, *kind) {
            assert!(
                !train.contains(&v),
                "{kind:?} test value `{v}` is also a training value"
            );
            assert!(
                !dev.contains(&v),
                "{kind:?} test value `{v}` is also a dev value"
            );
        }
    }
}

#[test]
fn test_phrasings_never_appear_in_dev() {
    let d = ds();
    let dev = skeletons(&d, Split::Dev, 20_000, 5);
    for s in skeletons(&d, Split::Test, 4000, 6) {
        assert!(
            !dev.contains(&s),
            "leak: test phrasing `{s}` is also a dev phrasing"
        );
    }
}

#[test]
fn errors_name_the_line() {
    let e = Dataset::parse("timer: {APP} please", "APP = a\nAPP? = b\nAPP! = c")
        .err()
        .unwrap();
    assert!(
        e.contains("templates.txt:1") && e.contains("not allowed"),
        "{e}"
    );
    let e = Dataset::parse("timer: @nope {DUR}", "").err().unwrap();
    assert!(e.contains("unknown list"), "{e}");
    let e = Dataset::parse("timer: [then {DUR}", "DUR = 5m\nDUR? = 6m\nDUR! = 7m")
        .err()
        .unwrap();
    assert!(e.contains("without `]`"), "{e}");
    let e = Dataset::parse("timer: {DUR", "DUR = 5m\nDUR? = 6m\nDUR! = 7m")
        .err()
        .unwrap();
    assert!(e.contains("stray"), "{e}");
    let e = Dataset::parse("timer: {DUR}", "DUR = 5m").err().unwrap();
    assert!(e.contains("no Dev values"), "{e}");
}
