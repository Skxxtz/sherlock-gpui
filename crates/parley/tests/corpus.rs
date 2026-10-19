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
    assert!(d.template_count(Split::Test) > 0);
}

#[test]
fn every_intent_has_train_and_test_examples() {
    let d = ds();
    let mut rng = Rng::new(1);
    for split in [Split::Train, Split::Test] {
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
        "DUR = 5m\nDUR! = 6m\nACTION = beep\nACTION! = ring",
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
fn same_seed_same_data() {
    let d = ds();
    let a = d.generate(Split::Train, Split::Train, 100, &mut Rng::new(7), true);
    let b = d.generate(Split::Train, Split::Train, 100, &mut Rng::new(7), true);
    assert_eq!(a, b);
}

#[test]
fn test_phrasings_never_appear_in_training() {
    let d = ds();
    let mut rng = Rng::new(3);
    let train: HashSet<String> = d
        .generate(Split::Train, Split::Train, 50_000, &mut rng, false)
        .iter()
        .map(|e| e.skeleton())
        .collect();
    for ex in d.generate(Split::Test, Split::Test, 2000, &mut rng, false) {
        assert!(
            !train.contains(&ex.skeleton()),
            "leak: `{}` is also a training phrasing",
            ex.skeleton()
        );
    }
}

#[test]
fn errors_name_the_line() {
    let e = Dataset::parse("timer: {APP} please", "APP = a\nAPP! = b")
        .err()
        .unwrap();
    assert!(
        e.contains("templates.txt:1") && e.contains("not allowed"),
        "{e}"
    );
    let e = Dataset::parse("timer: @nope {DUR}", "").err().unwrap();
    assert!(e.contains("unknown list"), "{e}");
    let e = Dataset::parse("timer: [then {DUR}", "DUR = 5m\nDUR! = 6m")
        .err()
        .unwrap();
    assert!(e.contains("without `]`"), "{e}");
    let e = Dataset::parse("timer: {DUR", "DUR = 5m\nDUR! = 6m")
        .err()
        .unwrap();
    assert!(e.contains("stray"), "{e}");
    let e = Dataset::parse("timer: {DUR}", "DUR = 5m").err().unwrap();
    assert!(e.contains("no Test values"), "{e}");
}
