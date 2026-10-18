//! Turns an input string into a flat list of tokens.

use crate::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    LParen,
    RParen,
    Comma,
}

/// A token plus its byte offset in the input.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub tok: Token,
    pub pos: usize,
}

pub fn tokenize(input: &str) -> Result<Vec<Spanned>, Error> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(pos, c)) = chars.peek() {
        let tok = match c {
            c if c.is_whitespace() => {
                chars.next();
                continue;
            }
            '0'..='9' | '.' => {
                let end = number_end(bytes, pos);
                let text = &input[pos..end];
                let n = text
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidNumber(text.to_string(), pos))?;
                while chars.peek().is_some_and(|&(i, _)| i < end) {
                    chars.next();
                }
                out.push(Spanned {
                    tok: Token::Num(n),
                    pos,
                });
                continue;
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut end = pos;
                while let Some(&(i, ch)) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        end = i + ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(Spanned {
                    tok: Token::Ident(input[pos..end].to_string()),
                    pos,
                });
                continue;
            }
            '*' => {
                chars.next();
                // `**` is an alias for `^`
                if chars.peek().is_some_and(|&(_, c)| c == '*') {
                    chars.next();
                    out.push(Spanned {
                        tok: Token::Caret,
                        pos,
                    });
                    continue;
                }
                out.push(Spanned {
                    tok: Token::Star,
                    pos,
                });
                continue;
            }
            c if superscript_digit(c).is_some() => {
                // `x²` / `x¹⁰` → `x ^ 2` / `x ^ 10`
                let mut n = 0.0;
                while let Some(d) = chars.peek().and_then(|&(_, c)| superscript_digit(c)) {
                    n = n * 10.0 + d as f64;
                    chars.next();
                }
                out.push(Spanned {
                    tok: Token::Caret,
                    pos,
                });
                out.push(Spanned {
                    tok: Token::Num(n),
                    pos,
                });
                continue;
            }
            '+' => Token::Plus,
            '-' | '−' => Token::Minus,
            '×' | '·' => Token::Star,
            '/' | '÷' => Token::Slash,
            '^' => Token::Caret,
            '%' => Token::Percent,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            other => return Err(Error::UnexpectedChar(other, pos)),
        };
        chars.next();
        out.push(Spanned { tok, pos });
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

    fn toks(s: &str) -> Vec<Token> {
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
        assert_eq!(toks("2e"), vec![Num(2.0), Ident("e".into())]);
        assert_eq!(toks("2ex"), vec![Num(2.0), Ident("ex".into())]);
    }

    #[test]
    fn expression() {
        assert_eq!(
            toks("2*sin(pi/4) + 1e3"),
            vec![
                Num(2.0),
                Star,
                Ident("sin".into()),
                LParen,
                Ident("pi".into()),
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
            vec![Ident("x".into()), Caret, Num(10.0), Plus, Num(1.0)]
        );
        assert_eq!(toks("2⁴⁹"), vec![Num(2.0), Caret, Num(49.0)]);
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
