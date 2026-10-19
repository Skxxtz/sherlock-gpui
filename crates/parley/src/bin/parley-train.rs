//! Trains the model and writes `model/parley.bin`, which the library embeds.
//! `cargo run -p parley --features training --bin parley-train --release [-- --errors [N]] [--test]`

use std::time::Instant;

use parley::SlotKind;
use parley::features::{self, Lexicon};
use parley::model::Model;
use parley::training::Rng;
use parley::training::calibrate;
use parley::training::corpus::{Dataset, Split};
use parley::training::evaluate::{by_intent, errors, evaluate, launcher};
use parley::training::trainer::{Settings, train};

const TRAIN_EXAMPLES: usize = 20_000;
const TEST_EXAMPLES: usize = 2_000;
const SEED: u64 = 42;

/// Decides whether a prediction is shown as a command.
type Gate<'a> = dyn Fn(&parley::model::Prediction, &[String]) -> bool + 'a;

fn main() {
    // `--errors [N]`: print up to N (default 12) wrong predictions per dev set
    // `--test`: also print the final test numbers (never with errors)
    let args: Vec<String> = std::env::args().collect();
    let show_errors = args.iter().any(|a| a == "--errors");
    let show_test = args.iter().any(|a| a == "--test");
    let show_errors_n = args
        .iter()
        .position(|a| a == "--errors")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or(12);
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
    let mut model = train(&examples, &lex, &settings, &mut rng);
    println!(
        "trained on {TRAIN_EXAMPLES} examples ({} templates), {} epochs, {:.2?}\n",
        ds.template_count(Split::Train),
        settings.epochs,
        t0.elapsed()
    );

    // confidence calibration (cross-validation on training templates only)
    let t0 = Instant::now();
    let (temp, nll1, nll) =
        calibrate::temperature(&ds, &lex, &settings, TRAIN_EXAMPLES / 2, &mut rng);
    model.temperature = temp;
    println!(
        "calibrated: temperature {temp:.2} (held-out NLL {nll1:.3} → {nll:.3}, {} folds, {:.2?})\n",
        calibrate::FOLDS,
        t0.elapsed()
    );

    // dev: held out from training; this is what we look at and tune against
    let row = |name: &str, r: parley::training::evaluate::Report| {
        println!(
            "{name:<36} {:>7.1}% {:>7.1}% {:>7.1}%",
            r.intent * 100.0,
            r.slot_f1 * 100.0,
            r.exact * 100.0
        );
    };
    println!(
        "{:<36} {:>8} {:>8} {:>8}",
        "dev set", "intent", "slot F1", "exact"
    );
    let sets = [
        ("seen phrasings, seen values", Split::Train, Split::Train),
        ("seen phrasings, DEV values", Split::Train, Split::Dev),
        ("DEV phrasings, seen values", Split::Dev, Split::Train),
        ("DEV phrasings, DEV values", Split::Dev, Split::Dev),
    ];
    let mut hardest = Vec::new();
    for (name, phr, val) in sets {
        let data = ds.generate(phr, val, TEST_EXAMPLES, &mut rng, false);
        row(name, evaluate(&model, &data, &lex));
        if show_errors {
            let mut errs = errors(&model, &data, &lex);
            errs.sort();
            errs.dedup_by(|a, b| a.0 == b.0);
            for (q, gold, pred) in errs.iter().take(show_errors_n) {
                println!("    ✗ {q:<40} want: {gold}\n      {:<40}  got: {pred}", "");
            }
        }
        hardest = data;
    }
    row(
        "  … same, without app lexicon",
        evaluate(&model, &hardest, &Lexicon::default()),
    );

    // test: held out from everything. Only shown with --test, never with errors:
    // looking at test mistakes and fixing them would make this number meaningless.
    if show_test {
        println!(
            "\n{:<36} {:>8} {:>8} {:>8}",
            "TEST (final number)", "intent", "slot F1", "exact"
        );
        let mut test_rng = Rng::new(SEED ^ 0x7e57);
        let data = ds.generate(
            Split::Test,
            Split::Train,
            TEST_EXAMPLES,
            &mut test_rng,
            false,
        );
        row("TEST phrasings, seen values", evaluate(&model, &data, &lex));
        let data = ds.generate(
            Split::Test,
            Split::Test,
            TEST_EXAMPLES,
            &mut test_rng,
            false,
        );
        row("TEST phrasings, TEST values", evaluate(&model, &data, &lex));
        row(
            "  … same, without app lexicon",
            evaluate(&model, &data, &Lexicon::default()),
        );
        let l = launcher(&model, &data, &lex, |p, w| {
            parley::validate::accept(p, w, &lex)
        });
        println!(
            "  launcher view (parse()): precision {:.1}%, coverage {:.1}%, false positives {:.1}%",
            l.precision * 100.0,
            l.coverage * 100.0,
            l.false_positive * 100.0
        );
        println!("  per intent (TEST phrasings, TEST values):");
        for (intent, n, r) in by_intent(&model, &data, &lex) {
            match intent {
                Some(i) => row(&format!("    {} ({n})", i.name()), r),
                // non-commands have no slots; "intent" = correctly left alone
                None => println!(
                    "{:<36} {:>7.1}% {:>8} {:>7.1}%",
                    format!("    none ({n})"),
                    r.intent * 100.0,
                    "–",
                    r.exact * 100.0
                ),
            }
        }
    }

    // what a launcher user would see: precision of shown commands, coverage,
    // and how often ordinary input gets a command
    let gates: [(&str, Box<Gate>); 3] = [
        ("any predicted command", Box::new(|p, _| p.intent.is_some())),
        (
            "+ confidence ≥ MIN_CONFIDENCE",
            Box::new(|p, _| p.intent.is_some() && p.confidence >= parley::MIN_CONFIDENCE),
        ),
        (
            "+ slot validation (= parse())",
            Box::new(|p, w| parley::validate::accept(p, w, &lex)),
        ),
    ];
    println!("\nlauncher view (DEV phrasings + DEV values):");
    println!(
        "  {:<32} {:>10} {:>10} {:>10}",
        "shown when", "precision", "coverage", "false pos"
    );
    for (name, gate) in &gates {
        let l = launcher(&model, &hardest, &lex, gate);
        println!(
            "  {name:<32} {:>9.1}% {:>9.1}% {:>9.1}%",
            l.precision * 100.0,
            l.coverage * 100.0,
            l.false_positive * 100.0
        );
    }

    // what each confidence threshold buys, with slot validation (as parse() does)
    println!("\nconfidence threshold + slot validation (DEV phrasings + DEV values):");
    println!(
        "  {:>9} {:>10} {:>10} {:>10}",
        "threshold", "precision", "coverage", "false pos"
    );
    for th in [0.0, 0.3, 0.5, 0.6, 0.7, 0.8, 0.9] {
        let l = launcher(&model, &hardest, &lex, |p, w| {
            p.intent.is_some_and(|i| {
                p.confidence >= th && parley::validate::plausible(i, w, &p.tags, &lex)
            })
        });
        println!(
            "  {:>9.2} {:>9.1}% {:>9.1}% {:>9.1}%",
            th,
            l.precision * 100.0,
            l.coverage * 100.0,
            l.false_positive * 100.0
        );
    }

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
