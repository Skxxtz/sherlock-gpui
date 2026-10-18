//! Streaming lexer: produces tokens on demand, names borrow from the input.

use crate::Error;

/// One piece of input. Aliases (`×`, `÷`, `**`, `−`) map to the same token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    /// `42`, `3.5`, `1e3`
    Num(f64),
    /// `sin`, `pi`, `x` (borrowed from the input)
    Ident(&'a str),
    /// `+`
    Plus,
    /// `-` `−`
    Minus,
    /// `*` `×` `·`
    Star,
    /// `/` `÷`
    Slash,
    /// `^` `**` (and superscripts)
    Caret,
    /// `%`
    Percent,
    /// `!`
    Bang,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `=`
    Eq,
    /// `⌊` … `⌋`
    LFloor,
    RFloor,
    /// `⌈` … `⌉`
    LCeil,
    RCeil,
    /// `|` (opens and closes abs)
    Bar,
    /// `√`
    Root,
    /// `∛`
    CubeRoot,
}

/// A token plus its byte offset in the input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spanned<'a> {
    /// The token.
    pub tok: Token<'a>,
    /// Byte offset where it starts.
    pub pos: usize,
}

/// Produces tokens one at a time from `src`.
pub struct Lexer<'a> {
    /// The input.
    src: &'a str,
    /// Byte offset of the next unread char.
    pos: usize,
    /// Second token of a superscript (`²` → `^`, `2`).
    pending: Option<Spanned<'a>>,
}

impl<'a> Lexer<'a> {
    /// Start at the beginning of `src`.
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src,
            pos: 0,
            pending: None,
        }
    }

    /// Next char without consuming it.
    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// Next token, or `None` at the end; skips whitespace.
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
            '⌊' => Token::LFloor,
            '⌋' => Token::RFloor,
            '⌈' => Token::LCeil,
            '⌉' => Token::RCeil,
            '|' => Token::Bar,
            '√' => Token::Root,
            '∛' => Token::CubeRoot,
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

/// `²` → 2; `None` for anything else.
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
