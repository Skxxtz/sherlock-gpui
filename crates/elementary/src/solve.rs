//! Solves `lhs = rhs` for one unknown.
//!
//! 1. Exact: if `lhs - rhs` is a polynomial of degree ≤ 2 in the unknown,
//!    use the closed formula (complex roots included).
//! 2. Numeric: otherwise scan [-1000, 1000] (finer near 0) for sign
//!    changes and bisect.
//!    Finds real roots where the function crosses zero.

use crate::Error;
use crate::compile::Program;
use crate::eval::eval_with;
use crate::func::{Func, constant};
use crate::parser::{Expr, Op};

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

/// Below this, a coefficient counts as zero.
const EPS: f64 = 1e-12;

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

// ---------- exact path ----------

/// Coefficients `[c0, c1, c2]` (c0 + c1·x + c2·x²).
type Poly = [f64; 3];

/// `Ok(None)` = not a polynomial of degree ≤ 2, try numerically.
fn poly(e: &Expr, var: &str) -> Result<Option<Poly>, Error> {
    if !e.contains(var) {
        return Ok(Some([eval_with(e, &[])?, 0.0, 0.0]));
    }
    let p = |e| poly(e, var);
    Ok(match e {
        Expr::Var(_) => Some([0.0, 1.0, 0.0]),
        // `x(a)` = `x * a`
        Expr::Call(name, args) if *name == var && args.len() == 1 => {
            let a = p(&args[0])?;
            a.and_then(|a| mul([0.0, 1.0, 0.0], a))
        }
        Expr::Neg(e) => p(e)?.map(|a| scale(a, -1.0)),
        Expr::Percent(e) => p(e)?.map(|a| scale(a, 0.01)),
        // calculator percent: `x + 10%` = x·1.1
        Expr::Binary(op @ (Op::Add | Op::Sub), l, r) if matches!(r.as_ref(), Expr::Percent(_)) => {
            let Expr::Percent(pct) = r.as_ref() else {
                unreachable!()
            };
            if pct.contains(var) {
                return Ok(None);
            }
            let k = eval_with(pct, &[])? / 100.0;
            p(l)?.map(|a| scale(a, if *op == Op::Add { 1.0 + k } else { 1.0 - k }))
        }
        Expr::Binary(op, l, r) => {
            let (Some(a), Some(b)) = (p(l)?, p(r)?) else {
                return Ok(None);
            };
            match op {
                Op::Add => Some(add(a, b)),
                Op::Sub => Some(add(a, scale(b, -1.0))),
                Op::Mul => mul(a, b),
                Op::Div if is_const(b) && b[0] != 0.0 => Some(scale(a, 1.0 / b[0])),
                Op::Pow if is_const(b) => match b[0] {
                    0.0 => Some([1.0, 0.0, 0.0]),
                    1.0 => Some(a),
                    2.0 => mul(a, a),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    })
}

/// No `x` or `x²` part.
fn is_const(p: Poly) -> bool {
    p[1] == 0.0 && p[2] == 0.0
}

/// `k · a`
fn scale(a: Poly, k: f64) -> Poly {
    a.map(|c| c * k)
}

/// `a + b`
fn add(a: Poly, b: Poly) -> Poly {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a · b`; `None` if the result is above degree 2.
fn mul(a: Poly, b: Poly) -> Option<Poly> {
    let mut r = [0.0; 5];
    for i in 0..3 {
        for j in 0..3 {
            r[i + j] += a[i] * b[j];
        }
    }
    (r[3] == 0.0 && r[4] == 0.0).then_some([r[0], r[1], r[2]])
}

/// Roots of `a·x² + b·x + c` by formula.
fn exact([c, b, a]: Poly) -> Result<Vec<Root>, Error> {
    let tiny = |v: f64| v.abs() < EPS;
    if tiny(a) && tiny(b) {
        return Err(if tiny(c) {
            Error::InfiniteSolutions
        } else {
            Error::NoSolution
        });
    }
    if tiny(a) {
        return Ok(vec![Root::Real(clean(-c / b))]);
    }
    let d = b * b - 4.0 * a * c;
    if tiny(d) {
        return Ok(vec![Root::Real(clean(-b / (2.0 * a)))]);
    }
    if d < 0.0 {
        let im = (-d).sqrt() / (2.0 * a).abs();
        return Ok(vec![Root::Complex {
            re: clean(-b / (2.0 * a)),
            im: clean(im),
        }]);
    }
    // numerically stable form
    let q = -0.5 * (b + b.signum() * d.sqrt());
    let mut r = [q / a, c / q];
    r.sort_by(f64::total_cmp);
    Ok(r.into_iter().map(|x| Root::Real(clean(x))).collect())
}

// ---------- numeric path ----------

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
fn clean(x: f64) -> f64 {
    let r = x.round();
    let x = if (x - r).abs() < 1e-9 * (1.0 + x.abs()) {
        r
    } else {
        x
    };
    if x == 0.0 { 0.0 } else { x }
}

#[cfg(test)]
mod tests {
    use super::Root::*;
    use super::*;
    use crate::solve_str;

    fn real(s: &str) -> Vec<f64> {
        solve_str(s)
            .unwrap_or_else(|e| panic!("{s:?}: {e}"))
            .roots
            .into_iter()
            .map(|r| match r {
                Real(x) => x,
                c => panic!("{s:?}: unexpected {c:?}"),
            })
            .collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-7)
    }

    #[test]
    fn linear() {
        assert_eq!(real("2x + 1 = 7"), vec![3.0]);
        assert_eq!(real("3(x - 2) = x"), vec![3.0]);
        assert_eq!(real("x / 4 = 2.5"), vec![10.0]);
        assert_eq!(real("x + 19% = 119"), vec![100.0]);
        assert_eq!(real("10 = 5y"), vec![2.0]);
    }

    #[test]
    fn quadratic() {
        assert_eq!(real("x² - 5x + 6 = 0"), vec![2.0, 3.0]);
        assert_eq!(real("x^2 = 9"), vec![-3.0, 3.0]);
        assert_eq!(real("(x - 1)² = 0"), vec![1.0]);
        assert_eq!(real("x(x + 2) = 0"), vec![-2.0, 0.0]);
        assert_eq!(
            solve_str("x² + 1 = 0").unwrap().roots,
            vec![Complex { re: 0.0, im: 1.0 }]
        );
    }

    #[test]
    fn numeric_fallback() {
        assert!(close(&real("2^x = 1024"), &[10.0]));
        assert!(close(&real("x³ = 27"), &[3.0]));
        assert!(close(&real("x³ - x = 0"), &[-1.0, 0.0, 1.0]));
        assert!(close(&real("sqrt(x) = 4"), &[16.0]));
        assert!(close(&real("ln(x) = 1"), &[std::f64::consts::E]));
        assert!(close(
            &real("cos(x) = 0")
                .iter()
                .filter(|r| r.abs() < 300.0)
                .copied()
                .collect::<Vec<_>>(),
            &[-270.0, -90.0, 90.0, 270.0]
        ));
    }

    #[test]
    fn rejects_poles() {
        // 1/x flips sign at 0 but has no root
        assert_eq!(solve_str("1/x = 0"), Err(Error::NoSolution));
    }

    #[test]
    fn errors() {
        assert_eq!(solve_str("x + 1"), Err(Error::NotAnEquation));
        assert_eq!(solve_str("2 = 3"), Err(Error::NoUnknown));
        assert_eq!(
            solve_str("x + y = 3"),
            Err(Error::TooManyUnknowns(vec!["x".into(), "y".into()]))
        );
        assert_eq!(solve_str("x + 1 = x + 2"), Err(Error::NoSolution));
        assert_eq!(solve_str("2x = x + x"), Err(Error::InfiniteSolutions));
    }
}
