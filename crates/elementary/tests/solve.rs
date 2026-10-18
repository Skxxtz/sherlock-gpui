use elementary::Root::*;
use elementary::{Error, solve_str};

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
