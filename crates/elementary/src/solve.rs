use crate::Error;
use crate::eval::func::{Func, constant};
use crate::syntax::ast::{Expr, Op};

pub mod compile;
pub mod poly;

use compile::Program;
use poly::{exact, poly};

/// One solution of the equation.
#[derive(Debug, Clone, PartialEq)]
pub enum Root {
    /// A real number.
    Real(f64),
    /// `re ± im·i` – stored once per conjugate pair.
    Complex { re: f64, im: f64 },
}

/// Result of `solve`.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    /// The unknown that was solved for.
    pub var: String,
    /// All roots found, sorted (real ones ascending).
    pub roots: Vec<Root>,
}

/// Scan grid as `(first, last, step)` in units of `step`: fine near 0,
/// coarser further out. Covers [-1000, 1000] with ~5600 points.
const GRID: &[(i64, i64, f64)] = &[
    (-1000, -101, 1.0),
    (-1000, -101, 0.1),
    (-1000, 999, 0.01),
    (100, 999, 0.1),
    (100, 1000, 1.0),
];

/// The scan points from `GRID`, ascending.
fn grid() -> impl Iterator<Item = f64> {
    // integer index × step, so e.g. 0 is hit exactly
    GRID.iter()
        .flat_map(|&(a, b, step)| (a..=b).map(move |k| k as f64 * step))
}

/// Solve `lhs = rhs`: exact formula if possible, else numeric search.
pub fn solve(lhs: Expr, rhs: Expr) -> Result<Solution, Error> {
    let var = unknown(&lhs, &rhs)?;
    let f = Expr::Binary(Op::Sub, Box::new(lhs), Box::new(rhs));

    let roots = match poly(&f, var)? {
        Some(p) => exact(p)?,
        None => numeric(&f, var)?,
    };
    Ok(Solution {
        var: var.to_string(),
        roots,
    })
}

/// The single free variable (a name that isn't a known constant).
fn unknown<'a>(l: &Expr<'a>, r: &Expr<'a>) -> Result<&'a str, Error> {
    let mut names = Vec::new();
    collect_vars(l, &mut names);
    collect_vars(r, &mut names);
    match names.as_slice() {
        [] => Err(Error::NoUnknown),
        [v] => Ok(v),
        _ => Err(Error::TooManyUnknowns(
            names.iter().map(|n| n.to_string()).collect(),
        )),
    }
}

/// Add every free name in `e` to `out` (no duplicates).
fn collect_vars<'a>(e: &Expr<'a>, out: &mut Vec<&'a str>) {
    let mut add = |n: &'a str| {
        if constant(n).is_none() && !out.contains(&n) {
            out.push(n);
        }
    };
    match e {
        Expr::Num(_) => {}
        Expr::Var(n) => add(n),
        Expr::Neg(e) | Expr::Percent(e) | Expr::Factorial(e) => collect_vars(e, out),
        Expr::Binary(_, l, r) => {
            collect_vars(l, out);
            collect_vars(r, out);
        }
        Expr::Call(name, args) => {
            // `x(2)` → x is a variable, not a function
            if Func::from_name(name).is_none() {
                add(name);
            }
            args.iter().for_each(|a| collect_vars(a, out))
        }
    }
}

/// Real roots of `f` via grid scan + bisection.
fn numeric(f: &Expr, var: &str) -> Result<Vec<Root>, Error> {
    let prog = std::cell::RefCell::new(Program::compile(f, var)?);
    let at = |x: f64| prog.borrow_mut().run(x);
    let mut roots: Vec<f64> = Vec::new();
    let mut push = |x: f64| {
        let x = clean(x);
        if !roots.iter().any(|r| (r - x).abs() < 1e-9 * (1.0 + x.abs())) {
            roots.push(x);
        }
    };

    let mut prev: Option<(f64, f64)> = None;
    for x in grid() {
        let Some(y) = at(x) else {
            prev = None; // undefined here: don't bridge across it
            continue;
        };
        if y == 0.0 {
            push(x);
        } else if let Some((px, py)) = prev
            && py != 0.0
            && py.signum() != y.signum()
            && let Some(r) = bisect(&at, px, x, py)
        {
            push(r);
        }
        prev = Some((x, y));
    }

    if roots.is_empty() {
        return Err(Error::NoSolution);
    }
    roots.sort_by(f64::total_cmp);
    Ok(roots.into_iter().map(Root::Real).collect())
}

/// Bisection on `[a, b]`; rejects poles (sign flips where |f| blows up).
fn bisect(at: &impl Fn(f64) -> Option<f64>, mut a: f64, mut b: f64, mut fa: f64) -> Option<f64> {
    let scale = fa.abs().max(at(b)?.abs()).max(1.0);
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if m == a || m == b {
            break;
        }
        let fm = at(m)?;
        if fm == 0.0 {
            return Some(m);
        }
        if fm.signum() == fa.signum() {
            (a, fa) = (m, fm);
        } else {
            b = m;
        }
    }
    let m = 0.5 * (a + b);
    (at(m)?.abs() <= 1e-6 * scale).then_some(m)
}

/// Snap float noise: `2.9999999999999996` → 3, `-0.0` → 0.
pub(crate) fn clean(x: f64) -> f64 {
    let r = x.round();
    let x = if (x - r).abs() < 1e-9 * (1.0 + x.abs()) {
        r
    } else {
        x
    };
    if x == 0.0 { 0.0 } else { x }
}
