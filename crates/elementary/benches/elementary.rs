use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Inputs both crates accept (meval uses radians, but cost is comparable).
const SHARED: &[(&str, &str)] = &[
    ("simple", "1 + 2"),
    ("arith", "(3 + 4) * 2 - 10 / 4 ^ 2"),
    ("funcs", "sqrt(16) + abs(-3) * max(1, 5, 3)"),
    ("trig", "sin(30) + cos(60) * tan(45)"),
    ("long", "1+2*3-4/5+6*7-8/9+10*11-12/13+14*15-16/17+18*19-20"),
];

fn eval(c: &mut Criterion) {
    for (name, expr) in SHARED {
        let mut g = c.benchmark_group(format!("eval/{name}"));
        g.bench_function("elementary", |b| {
            b.iter(|| elementary::eval_str(black_box(*expr)))
        });
        g.bench_function("meval", |b| b.iter(|| meval::eval_str(black_box(*expr))));
        g.finish();
    }

    // elementary-only syntax
    let mut g = c.benchmark_group("eval/extras");
    for expr in ["200 + 19%", "2pi(3+4)", "5! + 3²", "garbage text"] {
        g.bench_function(expr, |b| b.iter(|| elementary::eval_str(black_box(expr))));
    }
    g.finish();
}

fn solve(c: &mut Criterion) {
    let mut g = c.benchmark_group("solve");
    for expr in [
        "2x + 1 = 7",
        "x² - 5x + 6 = 0",
        "x³ - 6x² + 11x - 6 = 0",
        "2^x = 1024",
        "tan(x) = 1",
    ] {
        g.bench_function(expr, |b| b.iter(|| elementary::solve_str(black_box(expr))));
    }
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    targets = eval, solve
}
criterion_main!(benches);
