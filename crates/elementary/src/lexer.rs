//! Streaming lexer: produces tokens on demand, names borrow from the input.

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    Num(f64),
    Ident(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    LParen,
    RParen,
    Comma,
    Eq,
}

/// A token plus its byte offset in the input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spanned<'a> {
    pub tok: Token<'a>,
    pub pos: usize,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    /// Second token of a superscript (`²` → `^`, `2`).
    pending: Option<Spanned<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src,
            pos: 0,
            pending: None,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    pub fn next_token(&mut self) -> Result<Option<Spanned<'a>>, Error> {
        if let Some(t) = self.pending.take() {
            return Ok(Some(t));
        }
        while let Some(c) = self.peek_char().filter(|c| c.is_whitespace()) {
            self.pos += c.len_utf8();
        }
        let start = self.pos;
        let Some(c) = self.peek_char() else {
            return Ok(None);
        };
        let at = |tok| Ok(Some(Spanned { tok, pos: start }));

        let tok = match c {
            '0'..='9' | '.' => {
                let end = number_end(self.src.as_bytes(), start);
                let text = &self.src[start..end];
                let n = text
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidNumber(text.to_string(), start))?;
                self.pos = end;
                return at(Token::Num(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let rest = &self.src[start..];
                let len = rest
                    .char_indices()
                    .find(|&(_, ch)| {
                        !(ch.is_alphanumeric() || ch == '_') || superscript_digit(ch).is_some()
                    })
                    .map_or(rest.len(), |(i, _)| i);
                self.pos += len;
                return at(Token::Ident(&rest[..len]));
            }
            c if superscript_digit(c).is_some() => {
                // `x²` / `x¹⁰` → `x ^ 2` / `x ^ 10`
                let mut n = 0.0;
                while let Some(ch) = self.peek_char() {
                    let Some(d) = superscript_digit(ch) else {
                        break;
                    };
                    n = n * 10.0 + d as f64;
                    self.pos += ch.len_utf8();
                }
                self.pending = Some(Spanned {
                    tok: Token::Num(n),
                    pos: start,
                });
                return at(Token::Caret);
            }
            // `**` is an alias for `^`
            '*' if self.src[start + 1..].starts_with('*') => {
                self.pos += 2;
                return at(Token::Caret);
            }
            '*' | '×' | '·' => Token::Star,
            '+' => Token::Plus,
            '-' | '−' => Token::Minus,
            '/' | '÷' => Token::Slash,
            '^' => Token::Caret,
            '%' => Token::Percent,
            '!' => Token::Bang,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '=' => Token::Eq,
            other => return Err(Error::UnexpectedChar(other, start)),
        };
        self.pos += c.len_utf8();
        at(tok)
    }
}

/// Lex the whole input at once (tests and debugging).
pub fn tokenize(input: &str) -> Result<Vec<Spanned<'_>>, Error> {
    let mut lx = Lexer::new(input);
    let mut out = Vec::new();
    while let Some(t) = lx.next_token()? {
        out.push(t);
    }
    Ok(out)
}

fn superscript_digit(c: char) -> Option<u32> {
    match c {
        '⁰' => Some(0),
        '¹' => Some(1),
        '²' => Some(2),
        '³' => Some(3),
        '⁴'..='⁹' => Some(c as u32 - '⁴' as u32 + 4),
        _ => None,
    }
}

/// End byte of a number starting at `start`: digits, optional fraction,
/// optional exponent. An `e` not followed by digits is not consumed.
fn number_end(b: &[u8], start: usize) -> usize {
    let digits = |mut i: usize| {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut i = digits(start);
    if i < b.len() && b[i] == b'.' {
        i = digits(i + 1);
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let k = digits(j);
        if k > j {
            i = k;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::*;

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
}
