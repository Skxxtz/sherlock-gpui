use std::borrow::Cow;

/// `/x` and `""` unchanged, `~` / `~/x` → `$HOME[/x]`, `x` → `$HOME/x`.
pub fn expand_one<'a>(part: &'a str, home: &str) -> Cow<'a, str> {
    let rest = match part.as_bytes().first() {
        None | Some(b'/') => return Cow::Borrowed(part),
        Some(b'~') => match part[1..].strip_prefix('/') {
            Some(rest) => rest,
            None if part.len() == 1 => "",      // just "~"
            None => return Cow::Borrowed(part), // "~user": leave alone
        },
        _ => part,
    };

    let mut out = String::with_capacity(home.len() + 1 + rest.len());
    out.push_str(home);
    if !rest.is_empty() {
        out.push('/');
        out.push_str(rest);
    }
    Cow::Owned(out)
}

/// Words in a scoped command that should be treated as paths.
pub fn looks_like_path(w: &str) -> bool {
    w.starts_with('~') || w.starts_with("./") || w.contains('/')
}

/// Turns raw input into the value inserted into `exec`.
/// Default: unchanged, without allocating.
pub trait Finalize {
    fn finalize<'a>(&self, raw: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(raw)
    }
}
