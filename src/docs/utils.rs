pub const fn snake_len(s: &str) -> usize {
    let b = s.as_bytes();
    let mut n = b.len() + 3; // + ".md"
    let mut i = 1;
    while i < b.len() {
        if b[i].is_ascii_uppercase() {
            n += 1;
        }
        i += 1;
    }
    n
}

pub const fn snake_md<const N: usize>(s: &str) -> [u8; N] {
    let b = s.as_bytes();
    let mut out = [0u8; N];
    let (mut i, mut j) = (0, 0);
    while i < b.len() {
        if b[i].is_ascii_uppercase() {
            if i > 0 {
                out[j] = b'_';
                j += 1;
            }
            out[j] = b[i].to_ascii_lowercase();
        } else {
            out[j] = b[i];
        }
        i += 1;
        j += 1;
    }
    out[j] = b'.';
    out[j + 1] = b'm';
    out[j + 2] = b'd';
    out
}

#[macro_export]
macro_rules! snake_file {
    ($t:ident) => {{
        const NAME: &str = stringify!($t);
        const BYTES: [u8; $crate::docs::utils::snake_len(NAME)] =
            $crate::docs::utils::snake_md(NAME);
        // SAFETY: built only from ASCII bytes of a Rust identifier
        const FILE: &str = unsafe { ::core::str::from_utf8_unchecked(&BYTES) };
        FILE
    }};
}
