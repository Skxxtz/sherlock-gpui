//! Thorough accuracy evaluation: retrains with several seeds (spread of the
//! numbers), per-slot F1, error types (dev only), latency. Never writes the
//! model file.
//! `cargo run -p parley --features training --bin parley-eval --release`

use std::time::Instant;

use parley::features::Lexicon;
use parley::model::{self, Model, class_of, intent_of};
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
fn slot_f1(
    model: &Model,
    data: &[Example],
    lex: &Lexicon,
    oracle: bool,
) -> Vec<(SlotKind, f64, usize)> {
    SlotKind::ALL
        .iter()
        .map(|&k| {
            let (mut tp, mut fp, mut fn_, mut n) = (0, 0, 0, 0);
            for ex in data {
                let tags = if oracle {
                    model.extract(&ex.words, lex, ex.intent)
                } else {
                    model.predict(&ex.words, lex).tags
                };
                let g: Vec<_> = spans(&ex.tags).into_iter().filter(|s| s.2 == k).collect();
                let q: Vec<_> = spans(&tags).into_iter().filter(|s| s.2 == k).collect();
                n += g.len();
                tp += q.iter().filter(|s| g.contains(s)).count();
                fp += q.iter().filter(|s| !g.contains(s)).count();
                fn_ += g.iter().filter(|s| !q.contains(s)).count();
            }
            (k, 2.0 * tp as f64 / (2 * tp + fp + fn_).max(1) as f64, n)
        })
        .collect()
}

/// Share of examples whose slots are exactly right when the model is
/// given the correct intent: extraction quality on its own.
fn oracle_exact(model: &Model, data: &[Example], lex: &Lexicon) -> f64 {
    let ok = data
        .iter()
        .filter(|ex| model.extract(&ex.words, lex, ex.intent) == ex.tags)
        .count();
    ok as f64 / data.len().max(1) as f64
}

/// Exact-match errors split by cause: wrong intent, or right intent
/// but wrong slots. Shares of all examples.
fn error_split(model: &Model, data: &[Example], lex: &Lexicon) -> (f64, f64) {
    let (mut intent, mut slots) = (0, 0);
    for ex in data {
        let p = model.predict(&ex.words, lex);
        if p.intent != ex.intent {
            intent += 1;
        } else if p.tags != ex.tags {
            slots += 1;
        }
    }
    let n = data.len().max(1) as f64;
    (intent as f64 / n, slots as f64 / n)
}

fn class_name(c: usize) -> &'static str {
    intent_of(c).map_or("none", |i| i.name())
}

/// Intent confusion matrix (rows = gold, columns = predicted), in %
/// of the row. Aggregate counts only, no individual examples.
fn confusion(model: &Model, data: &[Example], lex: &Lexicon) {
    let mut m = [[0usize; model::CLASSES]; model::CLASSES];
    for ex in data {
        let p = model.predict(&ex.words, lex);
        m[class_of(ex.intent)][class_of(p.intent)] += 1;
    }
    print!("  {:<8}", "gold\\pred");
    for c in 0..model::CLASSES {
        print!("{:>8}", class_name(c));
    }
    println!();
    for (g, row) in m.iter().enumerate() {
        let n = row.iter().sum::<usize>().max(1) as f64;
        print!("  {:<8}", class_name(g));
        for &v in row {
            if v == 0 {
                print!("{:>8}", ".");
            } else {
                print!("{:>7.1}%", v as f64 / n * 100.0);
            }
        }
        println!("   ({:.0})", n);
    }
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
        "dev  extraction (oracle intent)",
        "test extraction (oracle intent)",
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
            oracle_exact(&model, &dev, &lex),
            oracle_exact(&model, &test, &lex),
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
    println!(
        "  {:<10} {:>14} {:>14} {:>14} {:>14}",
        "", "", "", "oracle intent", ""
    );
    println!(
        "  {:<10} {:>14} {:>14} {:>14} {:>14}",
        "slot", "dev", "test", "dev", "test"
    );
    let (fd, ft) = (
        slot_f1(&model, &dev, &lex, false),
        slot_f1(&model, &test, &lex, false),
    );
    let (od, ot) = (
        slot_f1(&model, &dev, &lex, true),
        slot_f1(&model, &test, &lex, true),
    );
    for (((k, a, na), (_, b, nb)), ((_, c, _), (_, d, _))) in
        fd.iter().zip(&ft).zip(od.iter().zip(&ot))
    {
        println!(
            "  {:<10} {:>6.1}% ({na:>4}) {:>6.1}% ({nb:>4}) {:>13.1}% {:>13.1}%",
            k.name(),
            a * 100.0,
            b * 100.0,
            c * 100.0,
            d * 100.0
        );
    }

    println!("\n== where exact-match errors come from (last seed, % of all examples) ==");
    for (name, data) in [("dev", &dev), ("test", &test)] {
        let (i, s) = error_split(&model, data, &lex);
        println!(
            "  {name:<5} wrong intent {:>5.1}%   right intent, wrong slots {:>5.1}%",
            i * 100.0,
            s * 100.0
        );
    }

    for (name, data) in [("DEV", &dev), ("TEST", &test)] {
        println!("\n== intent confusion on {name} (row %, last seed) ==");
        confusion(&model, data, &lex);
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
