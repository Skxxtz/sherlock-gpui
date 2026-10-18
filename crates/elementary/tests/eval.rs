use elementary::{Error, eval_str};

fn ev(s: &str) -> f64 {
    eval_str(s).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn arithmetic() {
    assert_eq!(ev("1 + 2 * 3"), 7.0);
    assert_eq!(ev("2^10"), 1024.0);
    assert_eq!(ev("-3²"), -9.0);
    assert_eq!(ev("2^-1"), 0.5);
    assert_eq!(ev("10 % 3"), 1.0);
    assert_eq!(ev("-7 % 3"), 2.0);
    assert!(close(ev("2pi"), std::f64::consts::TAU));
    assert_eq!(ev("2(3+4)"), 14.0);
}

#[test]
fn percent() {
    assert_eq!(ev("50%"), 0.5);
    assert_eq!(ev("200 + 10%"), 220.0);
    assert_eq!(ev("80 - 25%"), 60.0);
    assert_eq!(ev("200 * 15%"), 30.0);
    assert_eq!(ev("50% * 8"), 4.0);
    assert!(close(ev("100 + 19%"), 119.0));
}

#[test]
fn trig_degrees() {
    assert_eq!(ev("sin(90)"), 1.0);
    assert_eq!(ev("sin(180)"), 0.0);
    assert_eq!(ev("cos(180)"), -1.0);
    assert_eq!(ev("cos(90)"), 0.0);
    assert!(close(ev("cos(60)"), 0.5));
    assert!(close(ev("tan(45)"), 1.0));
    assert!(close(ev("asin(1)"), 90.0));
    assert_eq!(
        eval_str("tan(90)"),
        Err(Error::Domain("tan is undefined here"))
    );
}

#[test]
fn functions() {
    assert_eq!(ev("sqrt(16)"), 4.0);
    assert_eq!(ev("log(1000)"), 3.0);
    assert_eq!(ev("log(8, 2)"), 3.0);
    assert!(close(ev("ln(e)"), 1.0));
    assert_eq!(ev("max(1, 5, 3)"), 5.0);
    assert_eq!(ev("min(4, -2)"), -2.0);
    assert_eq!(ev("round(2.5)"), 3.0);
    assert!(close(ev("pi(2)"), std::f64::consts::TAU));
}

#[test]
fn factorial() {
    assert_eq!(ev("0!"), 1.0);
    assert_eq!(ev("5!"), 120.0);
    assert_eq!(ev("3!^2"), 36.0);
    assert_eq!(ev("-3!"), -6.0);
    assert!(matches!(eval_str("2.5!"), Err(Error::Domain(_))));
    assert!(matches!(eval_str("(-1)!"), Err(Error::Domain(_))));
    assert_eq!(eval_str("171!"), Err(Error::NotFinite));
}

#[test]
fn errors() {
    assert_eq!(eval_str("1/0"), Err(Error::NotFinite));
    assert_eq!(eval_str("sqrt(-1)"), Err(Error::NotFinite));
    assert_eq!(
        eval_str("foo + 1"),
        Err(Error::UnknownVariable("foo".into()))
    );
    assert_eq!(
        eval_str("foo(1)"),
        Err(Error::UnknownFunction("foo".into()))
    );
    assert!(matches!(
        eval_str("sin(1, 2)"),
        Err(Error::WrongArgCount { .. })
    ));
}
