use crate::{
    sherlock_msg,
    utils::errors::{SherlockMessage, types::ErrorKind},
};

/// Converts an ASCII string to title case in place, without allocating new memory.
///
/// Each word boundary (indicated by `-`, `_`, or ` `) causes the following
/// character to be capitalized. Separators are normalized to spaces.
///
/// # Example
/// ```
/// let mut s = String::from("catppuccin-mocha");
/// make_ascii_title_case(&mut s).unwrap();
/// assert_eq!(s, "Catppuccin Mocha");
/// ```
///
/// # Errors
/// Returns a [`SherlockMessage`] warning if the string contains non-ASCII characters,
/// as in-place byte mutation is only safe for ASCII input.
///
/// # Safety
/// This function uses [`String::as_bytes_mut`] which is unsafe because the compiler
/// cannot statically guarantee that arbitrary byte writes preserve valid UTF-8.
/// Safety is upheld here by:
/// - Guarding with [`str::is_ascii`] before entering the unsafe block, ensuring
///   all bytes are in `0x00..=0x7F`
/// - Only writing ASCII bytes (`b' '`, results of `make_ascii_uppercase/lowercase`),
///   all of which are valid single-byte UTF-8 sequences
/// - Never changing the length or capacity of the string
pub fn make_ascii_title_case(s: &mut str) -> Result<(), SherlockMessage> {
    if !s.is_ascii() {
        return Err(sherlock_msg!(
            Warning,
            ErrorKind::InvalidData,
            "Provided string is not ascii"
        ));
    }
    let bytes = unsafe { s.as_bytes_mut() };
    let mut capitalize_next = true;
    for b in bytes.iter_mut() {
        if *b == b'-' || *b == b'_' || *b == b' ' {
            *b = b' ';
            capitalize_next = true;
        } else if capitalize_next {
            b.make_ascii_uppercase();
            capitalize_next = false;
        } else {
            b.make_ascii_lowercase();
        }
    }
    Ok(())
}

/// Converts a string to title case in place, supporting full Unicode.
///
/// Word boundaries are detected at `-`, `_`, and ` `, which are normalized
/// to spaces. The first character of each word is uppercased using Unicode
/// aware casing rules, meaning characters like `ü` correctly become `Ü`.
///
/// # Note
/// This function always allocates a [`Vec`] to collect character indices
/// before mutating the string. Additionally, characters whose uppercase form
/// differs in byte length (e.g. `ß` → `SS`) may cause a reallocation via
/// [`String::replace_range`].
///
/// # Example
/// ```
/// let mut s = String::from("catppuccin-mocha");
/// make_title_case(&mut s);
/// assert_eq!(s, "Catppuccin Mocha");
///
/// let mut s = String::from("über-theme");
/// make_title_case(&mut s);
/// assert_eq!(s, "Über Theme");
/// ```
pub fn make_title_case(s: &mut String) {
    if s.is_ascii() {
        make_ascii_title_case(s).ok();
        return;
    }

    let mut capitalize_next = true;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    for (i, c) in chars.into_iter().rev() {
        if c == '-' || c == '_' || c == ' ' {
            unsafe {
                s.as_bytes_mut()[i] = b' ';
            }
            capitalize_next = true;
        } else if capitalize_next {
            if c.is_ascii() {
                unsafe {
                    s.as_bytes_mut()[i].make_ascii_uppercase();
                }
            } else {
                let upper: String = c.to_uppercase().collect();
                s.replace_range(i..i + c.len_utf8(), &upper);
            }
            capitalize_next = false;
        }
    }
}
