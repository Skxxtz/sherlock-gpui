use elementary::{
    Error,
    syntax::{ast::*, parser::*},
};

/// Compact S-expression form for readable assertions.
fn sx(e: &Expr) -> String {
    match e {
        Expr::Num(n) => n.to_string(),
        Expr::Var(v) => v.to_string(),
        Expr::Neg(e) => format!("(neg {})", sx(e)),
        Expr::Percent(e) => format!("(% {})", sx(e)),
        Expr::Factorial(e) => format!("(! {})", sx(e)),
        Expr::Binary(op, l, r) => {
            let o = match op {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div => "/",
                Op::Mod => "mod",
                Op::Pow => "^",
            };
            format!("({o} {} {})", sx(l), sx(r))
        }
        Expr::Call(f, a) => {
            let a: Vec<_> = a.iter().map(sx).collect();
            format!("({f} {})", a.join(" "))
        }
    }
}

fn p(s: &str) -> String {
    sx(&parse_tree(s).unwrap())
}

fn perr(s: &str) -> Error {
    parse_tree(s).unwrap_err()
}

#[test]
fn precedence() {
    assert_eq!(p("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(p("1 - 2 - 3"), "(- (- 1 2) 3)");
    assert_eq!(p("2 ^ 3 ^ 2"), "(^ 2 (^ 3 2))");
    assert_eq!(p("(1 + 2) * 3"), "(* (+ 1 2) 3)");
}

#[test]
fn unary() {
    assert_eq!(p("-3^2"), "(neg (^ 3 2))");
    assert_eq!(p("-3²"), "(neg (^ 3 2))");
    assert_eq!(p("2^-1"), "(^ 2 (neg 1))");
    assert_eq!(p("2 - -3"), "(- 2 (neg 3))");
    assert_eq!(p("+5"), "5");
}

#[test]
fn implicit_mul() {
    assert_eq!(p("2pi"), "(* 2 pi)");
    assert_eq!(p("2(3+4)"), "(* 2 (+ 3 4))");
    assert_eq!(p("(1)(2)"), "(* 1 2)");
    assert_eq!(p("2 sin(x)"), "(* 2 (sin x))");
    assert_eq!(p("1/2pi"), "(* (/ 1 2) pi)");
}

#[test]
fn percent_and_mod() {
    assert_eq!(p("50%"), "(% 50)");
    assert_eq!(p("10 % 3"), "(mod 10 3)");
    assert_eq!(p("200 + 10%"), "(+ 200 (% 10))");
    assert_eq!(p("200 * 15%"), "(* 200 (% 15))");
    assert_eq!(p("(10%)"), "(% 10)");
    assert_eq!(p("50% * 8"), "(* (% 50) 8)");
    assert_eq!(p("10 % (1+2)"), "(mod 10 (+ 1 2))");
}

#[test]
fn factorial() {
    assert_eq!(p("5!"), "(! 5)");
    assert_eq!(p("-3!"), "(neg (! 3))");
    assert_eq!(p("3!^2"), "(^ (! 3) 2)");
    assert_eq!(p("2^3!"), "(^ 2 (! 3))");
    assert_eq!(p("(1+2)!"), "(! (+ 1 2))");
}

#[test]
fn calls() {
    assert_eq!(p("max(1, 2, 3)"), "(max 1 2 3)");
    assert_eq!(p("f()"), "(f )");
    assert_eq!(p("sin(pi/2)^2"), "(^ (sin (/ pi 2)) 2)");
}

#[test]
fn equation() {
    let (l, r) = parse_equation("2x + 1 = 7").unwrap();
    assert_eq!((sx(&l), sx(&r)), ("(+ (* 2 x) 1)".into(), "7".into()));
    assert_eq!(parse_equation("1 + 2"), Err(Error::NotAnEquation));
    assert_eq!(parse_equation("x = 1 = 2"), Err(Error::UnexpectedToken(6)));
    assert_eq!(parse_equation("= 2"), Err(Error::UnexpectedToken(0)));
}

#[test]
fn errors() {
    assert_eq!(perr("1 +"), Error::UnexpectedEnd);
    assert_eq!(perr("(1 + 2"), Error::MissingParen);
    assert_eq!(perr("1 + )"), Error::UnexpectedToken(4));
    assert_eq!(perr("2 3"), Error::UnexpectedToken(2));
    assert_eq!(perr("max(1,)"), Error::UnexpectedToken(6));
}
