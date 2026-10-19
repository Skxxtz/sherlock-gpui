use crate::Error;
use crate::syntax::{
    ast::{Args, Expr, Op, Sink, TreeSink},
    lexer::{Lexer, Spanned, Token},
};

/// Parse a whole expression into `sink`.
pub fn parse<'a, S: Sink<'a>>(src: &'a str, sink: &mut S) -> Result<S::Out, Error> {
    let mut p = Parser::new(src, sink)?;
    let out = p.additive()?;
    p.expect_end()?;
    Ok(out)
}

pub fn parse_tree(src: &str) -> Result<Expr<'_>, Error> {
    parse(src, &mut TreeSink)
}

/// Parse `lhs = rhs`. Exactly one `=` is required.
pub fn parse_equation(src: &str) -> Result<(Expr<'_>, Expr<'_>), Error> {
    let mut sink = TreeSink;
    let mut p = Parser::new(src, &mut sink)?;
    let lhs = p.additive()?;
    match p.cur {
        None => return Err(Error::NotAnEquation),
        Some(Spanned { tok: Token::Eq, .. }) => p.bump()?,
        Some(s) => return Err(Error::UnexpectedToken(s.pos)),
    }
    let rhs = p.additive()?;
    p.expect_end()?;
    Ok((lhs, rhs))
}

// ---------- parser ----------

struct Parser<'a, 's, S> {
    lex: Lexer<'a>,
    cur: Option<Spanned<'a>>,
    next: Option<Spanned<'a>>,
    sink: &'s mut S,
}

impl<'a, 's, S: Sink<'a>> Parser<'a, 's, S> {
    fn new(src: &'a str, sink: &'s mut S) -> Result<Self, Error> {
        let mut lex = Lexer::new(src);
        let cur = lex.next_token()?;
        let next = lex.next_token()?;
        Ok(Parser {
            lex,
            cur,
            next,
            sink,
        })
    }

    fn bump(&mut self) -> Result<(), Error> {
        self.cur = self.next.take();
        if self.cur.is_some() {
            self.next = self.lex.next_token()?;
        }
        Ok(())
    }

    fn peek(&self) -> Option<Token<'a>> {
        self.cur.map(|s| s.tok)
    }

    fn eat(&mut self, tok: Token) -> Result<bool, Error> {
        if self.peek() == Some(tok) {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn err_here(&self) -> Error {
        match self.cur {
            Some(s) => Error::UnexpectedToken(s.pos),
            None => Error::UnexpectedEnd,
        }
    }

    fn expect_end(&self) -> Result<(), Error> {
        match self.cur {
            None => Ok(()),
            Some(s) => Err(Error::UnexpectedToken(s.pos)),
        }
    }

    /// `+` and `-`, the weakest level and the entry point.
    ///
    /// - `2 + 3` → `sink.binary(Add, 2, 3)`
    /// - left to right (loop): `10 - 4 - 3` = `(10 - 4) - 3` = 3
    /// - each side comes from `multiplicative`, so `1 + 2 * 3` gets
    ///   the finished `6` as its right side
    /// - percent rule lives in the sink: `200 + 10%` = 220
    ///
    /// Called for the whole input, inside `( … )`, for each function
    /// argument and for each side of an equation. Stops at anything that
    /// isn't `+` or `-` (`)`, `,`, `=`, end of input) and returns the value.
    fn additive(&mut self) -> Result<S::Out, Error> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Op::Add,
                Some(Token::Minus) => Op::Sub,
                _ => return Ok(lhs),
            };
            self.bump()?;
            let rhs = self.multiplicative()?;
            lhs = self.sink.binary(op, lhs, rhs)?;
        }
    }

    /// `*`, `/`, `%` (modulo) and implicit multiplication,
    /// one level above `additive`.
    ///
    /// - `2 * 3` → `sink.binary(Mul, 2, 3)`
    /// - left to right (loop): `8 / 2 / 2` = `(8 / 2) / 2` = 2
    /// - `10 % 3` → modulo; `postfix` already took any percent `%`,
    ///   so a `%` that reaches this level is always modulo
    /// - implicit: a name or `(` right after a value multiplies:
    ///   `2pi`, `2(3+4)`, `2 sin(x)`. A number does not: `2 3` is an error
    ///
    /// No `*`, `/`, `%`, name or `(` next → the value is passed up unchanged.
    fn multiplicative(&mut self) -> Result<S::Out, Error> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Op::Mul,
                Some(Token::Slash) => Op::Div,
                // postfix percent was already consumed in `postfix`,
                // so a `%` reaching here has an operand after it
                Some(Token::Percent) => Op::Mod,
                // implicit multiplication; `2 3` is rejected as a likely typo
                // (`|` is excluded: it could be a closing bar)
                Some(
                    Token::Ident(_)
                    | Token::LParen
                    | Token::LFloor
                    | Token::LCeil
                    | Token::Root
                    | Token::CubeRoot,
                ) => {
                    let rhs = self.unary()?;
                    lhs = self.sink.binary(Op::Mul, lhs, rhs)?;
                    continue;
                }
                _ => return Ok(lhs),
            };
            self.bump()?;
            let rhs = self.unary()?;
            lhs = self.sink.binary(op, lhs, rhs)?;
        }
    }

    /// Leading sign, one level above `multiplicative`.
    ///
    /// - `-5` → `sink.neg(5)`
    /// - `+5` → just 5
    /// - stacks: `--5` = 5, `2 - -3` = 5
    /// - binds weaker than `^` and `%`: `-2^2` = `-(2^2)` = -4
    ///
    /// Only a `-` at the start of a value is a sign. A `-` between two
    /// values never reaches here; `additive` takes it as subtraction.
    fn unary(&mut self) -> Result<S::Out, Error> {
        if self.eat(Token::Minus)? {
            let e = self.unary()?;
            return self.sink.neg(e);
        }
        if self.eat(Token::Plus)? {
            return self.unary();
        }
        self.postfix()
    }

    /// Postfix `%` (percent), one level above `power`.
    ///
    /// - `50%` → `sink.percent(50)` = 0.5
    /// - `10%%` → loops: `(10%)%`
    /// - only percent if no operand follows: `10 % 3` is left for
    ///   `multiplicative` as modulo
    /// - binds weaker than `^`: `2^2%` = `(2^2)%`
    ///
    /// No `%` next → the value is passed up unchanged.
    fn postfix(&mut self) -> Result<S::Out, Error> {
        let mut e = self.power()?;
        while self.peek() == Some(Token::Percent) && !starts_operand(self.next.map(|s| s.tok)) {
            self.bump()?;
            e = self.sink.percent(e)?;
        }
        Ok(e)
    }

    /// `^`, one level above `factorial`.
    ///
    /// - `2^3` → `sink.binary(Pow, 2, 3)`
    /// - right to left: the exponent is parsed by `unary`,
    ///   which comes back here, so `2^3^2` = `2^(3^2)` = 512
    /// - signed exponent works: `2^-1` = 0.5
    /// - binds tighter than a leading `-`: `-2^2` = `-(2^2)` = -4
    ///
    /// No `^` next → the value is passed up unchanged.
    fn power(&mut self) -> Result<S::Out, Error> {
        let base = self.factorial()?;
        if self.eat(Token::Caret)? {
            let exp = self.unary()?; // right-assoc, allows `2^-1`
            return self.sink.binary(Op::Pow, base, exp);
        }
        Ok(base)
    }

    /// Postfix `!`, one level above `primary`.
    /// Takes a value and applies every `!` directly after it.
    ///
    /// - `5!` → `sink.factorial(5)`
    /// - `3!!` → loops: `(3!)!`
    /// - binds tighter than `^`: `2^3!` = `2^(3!)`, `3!^2` = `(3!)^2`
    ///
    /// No `!` next → the value is passed up unchanged.
    fn factorial(&mut self) -> Result<S::Out, Error> {
        let mut e = self.primary()?;
        while self.eat(Token::Bang)? {
            e = self.sink.factorial(e)?;
        }
        Ok(e)
    }

    /// Strongest level: the smallest complete pieces of an expression.
    /// Every value starts here and is passed up through the weaker levels.
    ///
    /// - number: `42` → `sink.num`
    /// - name: `pi` → `sink.var`
    /// - call: `sin(…)` → name directly followed by `(` → `sink.call`
    /// - group: `( … )` → restarts at `additive`, so
    ///   anything goes inside the brackets
    /// - bracket functions: `⌊x⌋` floor, `⌈x⌉` ceil, `|x|` abs
    /// - prefix roots: `√x`, `∛x` take the next primary, so `√2²` = `(√2)²`
    ///
    /// Anything else (an operator, `)`, end of input) is an error:
    /// a value was expected here.
    fn primary(&mut self) -> Result<S::Out, Error> {
        match self.peek() {
            Some(Token::Num(n)) => {
                self.bump()?;
                self.sink.num(n)
            }
            Some(Token::Ident(name)) => {
                self.bump()?;
                if self.eat(Token::LParen)? {
                    let args = self.args()?;
                    self.sink.call(name, args)
                } else {
                    self.sink.var(name)
                }
            }
            Some(Token::LParen) => {
                self.bump()?;
                let e = self.additive()?;
                self.expect_rparen()?;
                Ok(e)
            }
            Some(Token::LFloor) => self.enclosed(Token::RFloor, "floor"),
            Some(Token::LCeil) => self.enclosed(Token::RCeil, "ceil"),
            Some(Token::Bar) => self.enclosed(Token::Bar, "abs"),
            Some(Token::Root) => self.prefix("sqrt"),
            Some(Token::CubeRoot) => self.prefix("cbrt"),
            _ => Err(self.err_here()),
        }
    }

    /// `⌊ … ⌋`-style: opening token, an expression, `close`, then `func` on it.
    fn enclosed(&mut self, close: Token, func: &'static str) -> Result<S::Out, Error> {
        self.bump()?;
        let e = self.additive()?;
        if !self.eat(close)? {
            return Err(self.err_here_or(Error::MissingParen));
        }
        self.sink.call(func, one(e))
    }

    /// `√x`-style: the symbol, then `func` on the next primary.
    fn prefix(&mut self, func: &'static str) -> Result<S::Out, Error> {
        self.bump()?;
        let e = self.primary()?;
        self.sink.call(func, one(e))
    }

    /// Unexpected token here, or `at_end` if the input ran out.
    fn err_here_or(&self, at_end: Error) -> Error {
        match self.cur {
            Some(s) => Error::UnexpectedToken(s.pos),
            None => at_end,
        }
    }

    /// Arguments after `name(`, including the closing `)`.
    fn args(&mut self) -> Result<Args<S::Out>, Error> {
        let mut args = Args::new();
        if self.eat(Token::RParen)? {
            return Ok(args);
        }
        loop {
            args.push(self.additive()?);
            if !self.eat(Token::Comma)? {
                break;
            }
        }
        self.expect_rparen()?;
        Ok(args)
    }

    fn expect_rparen(&mut self) -> Result<(), Error> {
        if self.eat(Token::RParen)? {
            Ok(())
        } else {
            Err(self.err_here_or(Error::MissingParen))
        }
    }
}

/// A one-element argument list.
fn one<T>(e: T) -> Args<T> {
    let mut a = Args::new();
    a.push(e);
    a
}

fn starts_operand(t: Option<Token>) -> bool {
    // `|` is excluded: in `|10%|` it closes, it doesn't start an operand
    matches!(
        t,
        Some(
            Token::Num(_)
                | Token::Ident(_)
                | Token::LParen
                | Token::LFloor
                | Token::LCeil
                | Token::Root
                | Token::CubeRoot
        )
    )
}
