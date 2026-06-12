/// A compact, zero-allocation character set covering U+0000–U+00FF (Latin-1).
///
/// Backed by a 256-bit flat bitset across four `u64` words.
/// Membership test is O(1) and branchless — three instructions per char.
///
/// Covers all common Western European characters:
///   ä ö ü Ä Ö Ü ß é è ê ë à â æ ç ñ ő ø þ and every ASCII character.
///
/// Any codepoint above U+00FF always returns `false`.
///
/// # Examples
/// ```
/// // Compile-time constant — zero runtime cost.
/// const GERMAN: CharSet = CharSet::new()
///     .with_range_char('a', 'z')
///     .with_range_char('A', 'Z')
///     .with_char('ä').with_char('ö').with_char('ü')
///     .with_char('Ä').with_char('Ö').with_char('Ü')
///     .with_char('ß');
///
/// // Runtime parse from regex-style class body.
/// let european = CharSet::parse(r"[a-zA-ZÀ-öø-ÿ]").unwrap();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CharSet {
    // w[0]: U+0000–U+003F
    // w[1]: U+0040–U+007F  (upper ASCII)
    // w[2]: U+0080–U+00BF  (Latin-1 control + punctuation block)
    // w[3]: U+00C0–U+00FF  (Latin-1 letters: À Á Â … ü ý þ ÿ)
    w: [u64; 4],
}

impl CharSet {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { w: [0u64; 4] }
    }

    pub const fn full_latin1() -> Self {
        Self { w: [u64::MAX; 4] }
    }

    /// Add a single codepoint (must be ≤ U+00FF).
    #[inline(always)]
    pub const fn with_char(mut self, c: char) -> Self {
        let cp = c as usize;
        debug_assert!(cp <= 0xFF, "codepoint above U+00FF");
        self.w[cp / 64] |= 1u64 << (cp % 64);
        self
    }

    /// Add a single raw byte (always ≤ U+00FF by construction).
    #[inline(always)]
    pub const fn with_byte(self, b: u8) -> Self {
        self.with_char(b as char)
    }

    /// Add an inclusive range of codepoints.
    #[inline(always)]
    pub const fn with_range_char(mut self, lo: char, hi: char) -> Self {
        debug_assert!((lo as u32) <= (hi as u32));
        debug_assert!((hi as u32) <= 0xFF);
        let mut cp = lo as usize;
        let end = hi as usize;
        loop {
            self.w[cp / 64] |= 1u64 << (cp % 64);
            if cp == end { break; }
            cp += 1;
        }
        self
    }

    /// Add an inclusive range of raw bytes.
    #[inline(always)]
    pub const fn with_range(self, lo: u8, hi: u8) -> Self {
        self.with_range_char(lo as char, hi as char)
    }

    #[inline(always)]
    pub const fn complement(self) -> Self {
        Self {
            w: [!self.w[0], !self.w[1], !self.w[2], !self.w[3]],
        }
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self {
            w: [
                self.w[0] | other.w[0],
                self.w[1] | other.w[1],
                self.w[2] | other.w[2],
                self.w[3] | other.w[3],
            ],
        }
    }

    #[inline(always)]
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            w: [
                self.w[0] & other.w[0],
                self.w[1] & other.w[1],
                self.w[2] & other.w[2],
                self.w[3] & other.w[3],
            ],
        }
    }

    /// O(1) branchless membership test.
    /// Any codepoint above U+00FF always returns `false`.
    #[inline(always)]
    pub fn contains(self, c: char) -> bool {
        let cp = c as u32;
        if cp > 0xFF {
            return false;
        }
        let cp = cp as usize;
        (self.w[cp / 64] >> (cp % 64)) & 1 == 1
    }

    /// Cheaper variant when you already have a raw byte (always in range).
    #[inline(always)]
    pub fn contains_byte(self, b: u8) -> bool {
        let cp = b as usize;
        (self.w[cp / 64] >> (cp % 64)) & 1 == 1
    }

    /// `true` if every `char` in the string is a member.
    #[inline]
    pub fn contains_all(self, s: &str) -> bool {
        s.chars().all(|c| self.contains(c))
    }

    /// `true` if every byte in the slice is a member.
    #[inline]
    pub fn contains_all_bytes(self, s: &[u8]) -> bool {
        s.iter().all(|&b| self.contains_byte(b))
    }

    /// Index of the first `char` not in the set, or `None`.
    #[inline]
    pub fn find_first_non_member(self, s: &str) -> Option<usize> {
        let mut offset = 0;
        for c in s.chars() {
            if !self.contains(c) {
                return Some(offset);
            }
            offset += c.len_utf8();
        }
        None
    }

    #[inline]
    pub fn len(self) -> u32 {
        self.w.iter().map(|w| w.count_ones()).sum()
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.w == [0u64; 4]
    }

    /// Parse a regex-style character class.
    ///
    /// Pass either the bare body `"a-zÄ-Ö"` or with brackets `"[a-zÄ-Ö]"`.
    ///
    /// Supported syntax:
    /// - `^`              leading negation
    /// - `a-z` / `À-ö`   inclusive range (either side may be non-ASCII Latin-1)
    /// - `\n \r \t \\ \] \^ \-`  escape sequences
    /// - any other char ≤ U+00FF  literal
    ///
    /// Returns `Err` for codepoints above U+00FF or reversed ranges.
    pub fn parse(pattern: &str) -> Result<Self, CharSetError> {
        // Work over `char`s so multi-byte UTF-8 sequences are decoded naturally.
        let mut chars: &[char] = &pattern.chars().collect::<Vec<_>>();

        // strip optional surrounding brackets
        if chars.first() == Some(&'[') && chars.last() == Some(&']') {
            chars = &chars[1..chars.len() - 1];
        }

        let negate = chars.first() == Some(&'^');
        if negate {
            chars = &chars[1..];
        }

        let mut set = CharSet::new();
        let mut i = 0;

        // Read one logical character (handling backslash escapes).
        // Advances `i` past the consumed input.
        let read_one = |chars: &[char], i: &mut usize| -> Option<Result<char, CharSetError>> {
            let &c = chars.get(*i)?;
            if c == '\\' {
                let escaped = match chars.get(*i + 1) {
                    Some(&'n')  => '\n',
                    Some(&'r')  => '\r',
                    Some(&'t')  => '\t',
                    Some(&'\\') => '\\',
                    Some(&']')  => ']',
                    Some(&'[')  => '[',
                    Some(&'^')  => '^',
                    Some(&'-')  => '-',
                    Some(&c)    => c,    // unknown escape → literal
                    None        => c,    // trailing backslash → literal backslash
                };
                *i += 2;
                Some(Ok(escaped))
            } else {
                *i += 1;
                Some(Ok(c))
            }
        };

        while i < chars.len() {
            let lo = match read_one(chars, &mut i) {
                None => break,
                Some(Err(e)) => return Err(e),
                Some(Ok(c)) => c,
            };

            if lo as u32 > 0xFF {
                return Err(CharSetError::OutOfRange(lo));
            }

            // Is the next raw char an unescaped `-` followed by a range end?
            let is_range = chars.get(i) == Some(&'-')
                && chars.get(i + 1).is_some()
                && chars.get(i + 1) != Some(&']');

            if is_range {
                i += 1; // consume `-`
                match read_one(chars, &mut i) {
                    None => {
                        // trailing dash — treat as two literals
                        set = set.with_char(lo).with_char('-');
                    }
                    Some(Err(e)) => return Err(e),
                    Some(Ok(hi)) => {
                        if hi as u32 > 0xFF {
                            return Err(CharSetError::OutOfRange(hi));
                        }
                        if (hi as u32) < (lo as u32) {
                            return Err(CharSetError::ReversedRange(lo, hi));
                        }
                        set = set.with_range_char(lo, hi);
                    }
                }
            } else {
                set = set.with_char(lo);
            }
        }

        if negate {
            set = set.complement();
        }

        Ok(set)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharSetError {
    /// Codepoint is above U+00FF and cannot be stored in this set.
    OutOfRange(char),
    /// Range end is less than range start, e.g. `z-a`.
    ReversedRange(char, char),
}

impl std::fmt::Display for CharSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharSetError::OutOfRange(c) =>
                write!(f, "codepoint U+{:04X} ('{c}') is above U+00FF", *c as u32),
            CharSetError::ReversedRange(lo, hi) =>
                write!(f, "reversed range '{lo}'-'{hi}' in pattern"),
        }
    }
}

impl std::error::Error for CharSetError {}

pub const ASCII_ALPHA: CharSet = CharSet::new()
    .with_range(b'a', b'z')
    .with_range(b'A', b'Z');

pub const ASCII_DIGIT: CharSet = CharSet::new().with_range(b'0', b'9');
pub const ASCII_ALNUM: CharSet = ASCII_ALPHA.union(ASCII_DIGIT);
pub const ASCII_WORD:  CharSet = ASCII_ALNUM.with_byte(b'_');

pub const ASCII_WHITESPACE: CharSet = CharSet::new()
    .with_byte(b' ').with_byte(b'\t').with_byte(b'\n').with_byte(b'\r');

/// All accented Latin letters in U+00C0–U+00FF (excludes × U+00D7 and ÷ U+00F7).
pub const LATIN_LETTERS: CharSet = ASCII_ALPHA
    .with_range_char('À', 'Ö')   // U+00C0–U+00D6
    .with_range_char('Ø', 'ö')   // U+00D8–U+00F6  (skips × U+00D7)
    .with_range_char('ø', 'ÿ');  // U+00F8–U+00FF  (skips ÷ U+00F7)

/// Identifier chars: ASCII word chars + all common accented letters.
pub const UNICODE_WORD: CharSet = ASCII_WORD.union(LATIN_LETTERS);


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_still_works() {
        assert!(ASCII_ALPHA.contains('z'));
        assert!(!ASCII_ALPHA.contains('5'));
    }

    #[test]
    fn latin1_letters() {
        for c in ['ä', 'ö', 'ü', 'Ä', 'Ö', 'Ü', 'ß', 'é', 'ñ', 'ç', 'ø', 'þ'] {
            assert!(LATIN_LETTERS.contains(c), "expected '{c}' in LATIN_LETTERS");
        }
        // operators that live in the same block must be excluded
        assert!(!LATIN_LETTERS.contains('×')); // U+00D7
        assert!(!LATIN_LETTERS.contains('÷')); // U+00F7
    }

    #[test]
    fn parse_latin1_range() {
        let s = CharSet::parse("[À-ÿ]").unwrap();
        assert!(s.contains('ö'));
        assert!(s.contains('Ü'));
        assert!(!s.contains('A'));
    }

    #[test]
    fn parse_mixed_range() {
        let s = CharSet::parse("[a-zA-ZÀ-öø-ÿ]").unwrap();
        assert!(s.contains('g'));
        assert!(s.contains('ä'));
        assert!(s.contains('ø'));
        assert!(!s.contains('5'));
        assert!(s.contains('×'));
    }

    #[test]
    fn parse_negation() {
        let s = CharSet::parse("[^a-z]").unwrap();
        assert!(s.contains('A'));
        assert!(s.contains('ö'));
        assert!(!s.contains('g'));
    }

    #[test]
    fn above_latin1_rejected() {
        assert_eq!(
            CharSet::parse("[a-zЯ]"),
            Err(CharSetError::OutOfRange('Я'))
        );
    }

    #[test]
    fn out_of_range_char_returns_false() {
        let s = CharSet::full_latin1();
        assert!(!s.contains('€')); // U+20AC
        assert!(!s.contains('😀'));
    }

    #[test]
    fn popcount() {
        assert_eq!(ASCII_DIGIT.len(), 10);
        assert_eq!(ASCII_ALPHA.len(), 52);
        // À–ÿ is 64 codepoints minus × and ÷ = 62
        assert_eq!(LATIN_LETTERS.len(), 52 + 62);
    }

    #[test]
    fn find_first_non_member() {
        let s = CharSet::parse("[a-zä-ö]").unwrap();
        let input = "helloöwörld!";
        let idx = s.find_first_non_member(input);
        assert_eq!(input[idx.unwrap()..].chars().next(), Some('!'));
    }

    #[test]
    fn struct_size() {
        assert_eq!(std::mem::size_of::<CharSet>(), 32);
    }
}
