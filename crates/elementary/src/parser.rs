//! Turns tokens into an expression tree (precedence climbing).
//!
//! Precedence, lowest to highest:
//! 1. `+ -`
//! 2. `* /`, `%` (modulo), implicit multiplication (`2pi`, `2(3+4)`)
//! 3. unary `- +`
//! 4. postfix `%` (percent)
//! 5. `^` (right-assoc, exponent may be unary: `2^-1`)
//! 6. numbers, names, calls, `( … )`
//!
//! `%` is modulo when an operand follows it (`10 % 3`), otherwise percent
//! (`10%`, `10%)`, `10% * 2`). So `-3^2` = `-9`.

use crate::Error;
use crate::lexer::{Spanned, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Percent(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

pub fn parse(tokens: &[Spanned]) -> Result<Expr, Error> {
    let mut p = Parser { t: tokens, i: 0 };
    let e = p.additive()?;
    match p.t.get(p.i) {
        None => Ok(e),
        Some(s) => Err(Error::UnexpectedToken(s.pos)),
    }
}

struct Parser<'a> {
    t: &'a [Spanned],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    fn peek_at(&self, n: usize) -> Option<&Token> {
        self.t.get(self.i + n).map(|s| &s.tok)
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == Some(tok) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn err_here(&self) -> Error {
        match self.t.get(self.i) {
            Some(s) => Error::UnexpectedToken(s.pos),
            None => Error::UnexpectedEnd,
        }
    }

    fn additive(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Op::Add,
                Some(Token::Minus) => Op::Sub,
                _ => return Ok(lhs),
            };
            self.i += 1;
            let rhs = self.multiplicative()?;
            lhs = bin(op, lhs, rhs);
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Op::Mul,
                Some(Token::Slash) => Op::Div,
                // postfix percent was already consumed in `postfix`,
                // so a `%` reaching here has an operand after it
                Some(Token::Percent) => Op::Mod,
                // implicit multiplication; `2 3` is rejected as a likely typo
                Some(Token::Ident(_) | Token::LParen) => {
                    let rhs = self.unary()?;
                    lhs = bin(Op::Mul, lhs, rhs);
                    continue;
                }
                _ => return Ok(lhs),
            };
            self.i += 1;
            let rhs = self.unary()?;
            lhs = bin(op, lhs, rhs);
        }
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.eat(&Token::Minus) {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat(&Token::Plus) {
            return self.unary();
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut e = self.power()?;
        while self.peek() == Some(&Token::Percent) && !starts_operand(self.peek_at(1)) {
            self.i += 1;
            e = Expr::Percent(Box::new(e));
        }
        Ok(e)
    }

    fn power(&mut self) -> Result<Expr, Error> {
        let base = self.primary()?;
        if self.eat(&Token::Caret) {
            let exp = self.unary()?; // right-assoc, allows `2^-1`
            return Ok(bin(Op::Pow, base, exp));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let Some(tok) = self.peek().cloned() else {
            return Err(Error::UnexpectedEnd);
        };
        match tok {
            Token::Num(n) => {
                self.i += 1;
                Ok(Expr::Num(n))
            }
            Token::Ident(name) => {
                self.i += 1;
                if self.eat(&Token::LParen) {
                    let args = self.args()?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Token::LParen => {
                self.i += 1;
                let e = self.additive()?;
                self.expect_rparen()?;
                Ok(e)
            }
            _ => Err(self.err_here()),
        }
    }

    /// Arguments after `name(`, including the closing `)`.
    fn args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if self.eat(&Token::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.additive()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect_rparen()?;
        Ok(args)
    }

    fn expect_rparen(&mut self) -> Result<(), Error> {
        if self.eat(&Token::RParen) {
            Ok(())
        } else {
            Err(match self.t.get(self.i) {
                Some(s) => Error::UnexpectedToken(s.pos),
                None => Error::MissingParen,
            })
        }
    }
}

fn starts_operand(t: Option<&Token>) -> bool {
    matches!(t, Some(Token::Num(_) | Token::Ident(_) | Token::LParen))
}

fn bin(op: Op, l: Expr, r: Expr) -> Expr {
    Expr::Binary(op, Box::new(l), Box::new(r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    /// Compact S-expression form for readable assertions.
    fn sx(e: &Expr) -> String {
        match e {
            Expr::Num(n) => n.to_string(),
            Expr::Var(v) => v.clone(),
            Expr::Neg(e) => format!("(neg {})", sx(e)),
            Expr::Percent(e) => format!("(% {})", sx(e)),
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
        sx(&parse(&tokenize(s).unwrap()).unwrap())
    }

    fn perr(s: &str) -> Error {
        parse(&tokenize(s).unwrap()).unwrap_err()
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
    fn calls() {
        assert_eq!(p("max(1, 2, 3)"), "(max 1 2 3)");
        assert_eq!(p("f()"), "(f )");
        assert_eq!(p("sin(pi/2)^2"), "(^ (sin (/ pi 2)) 2)");
    }

    #[test]
    fn errors() {
        assert_eq!(perr("1 +"), Error::UnexpectedEnd);
        assert_eq!(perr("(1 + 2"), Error::MissingParen);
        assert_eq!(perr("1 + )"), Error::UnexpectedToken(4));
        assert_eq!(perr("2 3"), Error::UnexpectedToken(2));
        assert_eq!(perr("max(1,)"), Error::UnexpectedToken(6));
    }
}
