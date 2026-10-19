use elementary::Error;
use elementary::syntax::lexer::{Token, Token::*, tokenize};

fn toks(s: &str) -> Vec<Token<'_>> {
    tokenize(s).unwrap().into_iter().map(|t| t.tok).collect()
}

#[test]
fn numbers() {
    assert_eq!(
        toks("42 3.5 .5 1e3 2E-2"),
        vec![Num(42.0), Num(3.5), Num(0.5), Num(1000.0), Num(0.02)]
    );
}

#[test]
fn exponent_without_digits_stays_ident() {
    assert_eq!(toks("2e"), vec![Num(2.0), Ident("e")]);
    assert_eq!(toks("2ex"), vec![Num(2.0), Ident("ex")]);
}

#[test]
fn expression() {
    assert_eq!(
        toks("2*sin(pi/4) + 1e3"),
        vec![
            Num(2.0),
            Star,
            Ident("sin"),
            LParen,
            Ident("pi"),
            Slash,
            Num(4.0),
            RParen,
            Plus,
            Num(1000.0)
        ]
    );
}

#[test]
fn operator_aliases() {
    assert_eq!(
        toks("2**3 × 4 ÷ 2 − 1"),
        vec![
            Num(2.0),
            Caret,
            Num(3.0),
            Star,
            Num(4.0),
            Slash,
            Num(2.0),
            Minus,
            Num(1.0)
        ]
    );
}

#[test]
fn superscripts() {
    assert_eq!(toks("3²"), vec![Num(3.0), Caret, Num(2.0)]);
    assert_eq!(
        toks("x¹⁰+1"),
        vec![Ident("x"), Caret, Num(10.0), Plus, Num(1.0)]
    );
    assert_eq!(toks("2⁴⁹"), vec![Num(2.0), Caret, Num(49.0)]);
}

#[test]
fn bang() {
    assert_eq!(toks("5!"), vec![Num(5.0), Bang]);
}

#[test]
fn minus_is_never_part_of_number() {
    assert_eq!(toks("2-3"), vec![Num(2.0), Minus, Num(3.0)]);
}

#[test]
fn positions() {
    let t = tokenize("1 + ab").unwrap();
    assert_eq!(t.iter().map(|s| s.pos).collect::<Vec<_>>(), vec![0, 2, 4]);
}

#[test]
fn errors() {
    assert_eq!(tokenize("1 $ 2"), Err(Error::UnexpectedChar('$', 2)));
    assert_eq!(tokenize("."), Err(Error::InvalidNumber(".".into(), 0)));
}
