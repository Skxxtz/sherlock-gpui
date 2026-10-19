//! Thorough accuracy evaluation: retrains with several seeds (spread of the
//! numbers), per-slot F1, error types (dev only), latency. Never writes the
//! model file.
//! `cargo run -p parley --features training --bin parley-eval --release`

use std::time::Instant;

use parley::features::Lexicon;
use parley::model::Model;
use parley::training::Rng;
use parley::training::calibrate;
use parley::training::corpus::{Dataset, Example, Split};
use parley::training::evaluate::{evaluate, launcher};
use parley::training::trainer::{Settings, train};
use parley::{SlotKind, Tag, validate};

const SEEDS: [u64; 5] = [42, 7, 1234, 99, 2026];
const N_TRAIN: usize = 20_000;
const N_EVAL: usize = 2_000;

fn spans(tags: &[Tag]) -> Vec<(usize, usize, SlotKind)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tags.len() {
        if let Tag::B(k) = tags[i] {
            let mut j = i + 1;
            while j < tags.len() && tags[j] == Tag::I(k) {
                j += 1;
            }
            out.push((i, j, k));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// F1 per slot kind.
fn slot_f1(model: &Model, data: &[Example], lex: &Lexicon) -> Vec<(SlotKind, f64, usize)> {
    SlotKind::ALL
        .iter()
        .map(|&k| {
            let (mut tp, mut fp, mut fn_, mut n) = (0, 0, 0, 0);
            for ex in data {
                let p = model.predict(&ex.words, lex);
                let g: Vec<_> = spans(&ex.tags).into_iter().filter(|s| s.2 == k).collect();
                let q: Vec<_> = spans(&p.tags).into_iter().filter(|s| s.2 == k).collect();
                n += g.len();
                tp += q.iter().filter(|s| g.contains(s)).count();
                fp += q.iter().filter(|s| !g.contains(s)).count();
                fn_ += g.iter().filter(|s| !q.contains(s)).count();
            }
            (k, 2.0 * tp as f64 / (2 * tp + fp + fn_).max(1) as f64, n)
        })
        .collect()
}

/// Error categories for wrong examples.
fn error_types(model: &Model, data: &[Example], lex: &Lexicon) -> Vec<(&'static str, usize)> {
    let mut c = [0usize; 5];
    for ex in data {
        let p = model.predict(&ex.words, lex);
        let (g, q) = (spans(&ex.tags), spans(&p.tags));
        if p.intent == ex.intent && g == q {
            continue;
        }
        let i = if p.intent != ex.intent {
            0
        } else {
            let same_kinds = g.iter().map(|s| s.2).collect::<Vec<_>>()
                == q.iter().map(|s| s.2).collect::<Vec<_>>();
            if same_kinds {
                1 // right slots, wrong boundaries
            } else if q.len() < g.len() {
                2 // slot missing
            } else if q.len() > g.len() {
                3 // extra slot
            } else {
                4 // wrong slot kind
            }
        };
        c[i] += 1;
    }
    vec![
        ("wrong intent", c[0]),
        ("right slots, wrong boundaries", c[1]),
        ("slot missing", c[2]),
        ("extra slot", c[3]),
        ("wrong slot kind", c[4]),
    ]
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1).max(1) as f64;
    (m, var.sqrt())
}

fn main() {
    let ds = Dataset::bundled().expect("data files");
    let lex = Lexicon::with_apps(ds.values(SlotKind::App));
    let settings = Settings::default();

    // metric name → values over seeds
    let names = [
        "dev  exact (dev phr + dev val)",
        "dev  intent",
        "dev  slot F1",
        "test exact (test phr + test val)",
        "test exact (test phr, seen val)",
        "test intent",
        "test slot F1",
        "test precision (parse())",
        "test coverage  (parse())",
        "test false positives",
    ];
    let mut values: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    let mut last: Option<(Model, Vec<Example>, Vec<Example>)> = None;

    for (n, seed) in SEEDS.iter().enumerate() {
        let t0 = Instant::now();
        let mut rng = Rng::new(*seed);
        let examples = ds.generate(Split::Train, Split::Train, N_TRAIN, &mut rng, true);
        let mut model = train(&examples, &lex, &settings, &mut rng);
        model.temperature = calibrate::temperature(&ds, &lex, &settings, N_TRAIN / 2, &mut rng).0;

        let dev = ds.generate(Split::Dev, Split::Dev, N_EVAL, &mut rng, false);
        let test = ds.generate(Split::Test, Split::Test, N_EVAL, &mut rng, false);
        let test_seen = ds.generate(Split::Test, Split::Train, N_EVAL, &mut rng, false);
        let (d, t, ts) = (
            evaluate(&model, &dev, &lex),
            evaluate(&model, &test, &lex),
            evaluate(&model, &test_seen, &lex),
        );
        let l = launcher(&model, &test, &lex, |p, w| validate::accept(p, w, &lex));
        for (k, v) in [
            d.exact,
            d.intent,
            d.slot_f1,
            t.exact,
            ts.exact,
            t.intent,
            t.slot_f1,
            l.precision,
            l.coverage,
            l.false_positive,
        ]
        .into_iter()
        .enumerate()
        {
            values[k].push(v * 100.0);
        }
        eprintln!("seed {}/{} done ({:.1?})", n + 1, SEEDS.len(), t0.elapsed());
        last = Some((model, dev, test));
    }

    println!(
        "\n== stability over {} training seeds (mean ± sd) ==",
        SEEDS.len()
    );
    for (name, v) in names.iter().zip(&values) {
        let (m, sd) = mean_sd(v);
        let (lo, hi) = v
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
        println!("  {name:<34} {m:>6.1}% ± {sd:>4.1}   (min {lo:.1}, max {hi:.1})");
    }

    let (model, dev, test) = last.unwrap();
    println!("\n== slot F1 per kind (last seed) ==");
    println!("  {:<10} {:>14} {:>14}", "slot", "dev", "test");
    let (fd, ft) = (slot_f1(&model, &dev, &lex), slot_f1(&model, &test, &lex));
    for ((k, a, na), (_, b, nb)) in fd.iter().zip(&ft) {
        println!(
            "  {:<10} {:>6.1}% ({na:>4}) {:>6.1}% ({nb:>4})",
            k.name(),
            a * 100.0,
            b * 100.0
        );
    }

    println!("\n== error types on DEV (test errors are never inspected) ==");
    let errs = error_types(&model, &dev, &lex);
    let total: usize = errs.iter().map(|e| e.1).sum();
    for (name, n) in errs {
        println!(
            "  {name:<32} {n:>5}  ({:>4.1}% of errors)",
            n as f64 / total.max(1) as f64 * 100.0
        );
    }

    println!("\n== latency (predict + validate, release build) ==");
    let mut times: Vec<f64> = Vec::new();
    for ex in dev.iter().chain(&test) {
        let t0 = Instant::now();
        let p = model.predict(&ex.words, &lex);
        let _ = std::hint::black_box(validate::accept(&p, &ex.words, &lex));
        times.push(t0.elapsed().as_secs_f64() * 1e6);
    }
    times.sort_by(f64::total_cmp);
    let q = |p: f64| times[((times.len() - 1) as f64 * p) as usize];
    println!(
        "  p50 {:.1} µs   p90 {:.1} µs   p99 {:.1} µs   max {:.1} µs",
        q(0.5),
        q(0.9),
        q(0.99),
        times[times.len() - 1]
    );
}
