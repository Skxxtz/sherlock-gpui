//! Trains the model and writes `model/parley.bin`, which the library embeds.
//! `cargo run -p parley --features training --bin parley-train --release`

use std::time::Instant;

use parley::SlotKind;
use parley::features::{self, Lexicon};
use parley::model::Model;
use parley::training::Rng;
use parley::training::corpus::{Dataset, Split};
use parley::training::evaluate::evaluate;
use parley::training::trainer::{Settings, train};

const TRAIN_EXAMPLES: usize = 20_000;
const TEST_EXAMPLES: usize = 2_000;
const SEED: u64 = 42;

fn main() {
    let ds = Dataset::bundled().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
    let mut rng = Rng::new(SEED);
    // at runtime Sherlock knows the installed apps; simulate that here
    let lex = Lexicon::with_apps(ds.values(SlotKind::App));
    let settings = Settings::default();

    let examples = ds.generate(Split::Train, Split::Train, TRAIN_EXAMPLES, &mut rng, true);
    let t0 = Instant::now();
    let model = train(&examples, &lex, &settings, &mut rng);
    println!(
        "trained on {TRAIN_EXAMPLES} examples ({} templates), {} epochs, {:.2?}\n",
        ds.template_count(Split::Train),
        settings.epochs,
        t0.elapsed()
    );

    // honest evaluation: phrasings and values the model never saw
    let sets = [
        ("seen phrasings, seen values", Split::Train, Split::Train),
        ("seen phrasings, NEW values", Split::Train, Split::Test),
        ("NEW phrasings, seen values", Split::Test, Split::Train),
        ("NEW phrasings, NEW values", Split::Test, Split::Test),
    ];
    println!(
        "{:<36} {:>8} {:>8} {:>8}",
        "test set", "intent", "slot F1", "exact"
    );
    let mut hardest = Vec::new();
    for (name, phr, val) in sets {
        let data = ds.generate(phr, val, TEST_EXAMPLES, &mut rng, false);
        let r = evaluate(&model, &data, &lex);
        println!(
            "{name:<36} {:>7.1}% {:>7.1}% {:>7.1}%",
            r.intent * 100.0,
            r.slot_f1 * 100.0,
            r.exact * 100.0
        );
        hardest = data;
    }
    let r = evaluate(&model, &hardest, &Lexicon::default());
    println!(
        "{:<36} {:>7.1}% {:>7.1}% {:>7.1}%",
        "  … same, without app lexicon",
        r.intent * 100.0,
        r.slot_f1 * 100.0,
        r.exact * 100.0
    );

    // speed
    let queries: Vec<Vec<String>> = hardest.iter().map(|e| e.words.clone()).collect();
    let t0 = Instant::now();
    let mut sink = 0;
    for _ in 0..10 {
        for q in &queries {
            sink += model.predict(q, &lex).tags.len();
        }
    }
    let per = t0.elapsed() / (10 * queries.len()) as u32;
    println!("\nspeed: {per:.2?} per query [{sink}]");

    // write
    let bytes = model.to_bytes();
    assert_eq!(
        Model::from_bytes(&bytes).as_ref(),
        Ok(&model),
        "round trip failed"
    );
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/model/parley.bin");
    std::fs::write(path, &bytes).expect("write model");
    println!(
        "wrote model/parley.bin ({:.1} KB)",
        bytes.len() as f64 / 1024.0
    );

    // examples
    println!("\nexamples:");
    for q in [
        "new 20m timer then notify me",
        "make me a 45 secs timer",
        "remind me in 10 minutes to water the plants",
        "at 7am tomorrow remind me to email bob",
        "bring the volume down to 35%",
        "fire up steam",
        "find wayland layer shell on the web",
        "firefox",
        "play some music",
    ] {
        let p = model.predict(&features::words(q), &lex);
        let words: Vec<&str> = q.split_whitespace().collect();
        let mut slots = Vec::new();
        for (w, t) in words.iter().zip(&p.tags) {
            match t {
                parley::Tag::B(k) => slots.push(format!("{}=\"{w}", k.name())),
                parley::Tag::I(_) => {
                    let s: &mut String = slots.last_mut().unwrap();
                    s.push(' ');
                    s.push_str(w);
                }
                parley::Tag::O => {}
            }
        }
        let slots: Vec<String> = slots.into_iter().map(|s| s + "\"").collect();
        let intent = p.intent.map_or("none", |i| i.name());
        println!(
            "  {q:<44} → {intent:<7} {:>4.0}%  {}",
            p.confidence * 100.0,
            slots.join(" ")
        );
    }
}
