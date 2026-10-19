use crate::{
    Root,
    error::Error,
    eval::eval_with,
    solve::clean,
    syntax::ast::{Expr, Op},
};

/// Below this, a coefficient counts as zero.
const EPS: f64 = 1e-12;

/// Coefficients `[c0, c1, c2]` (c0 + c1·x + c2·x²).
type Poly = [f64; 3];

/// `Ok(None)` = not a polynomial of degree ≤ 2, try numerically.
pub fn poly(e: &Expr, var: &str) -> Result<Option<Poly>, Error> {
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
pub fn exact([c, b, a]: Poly) -> Result<Vec<Root>, Error> {
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
