#[allow(unused)]
pub trait TrimInPlace {
    fn trim_in_place(&mut self);
}

impl TrimInPlace for String {
    fn trim_in_place(&mut self) {
        let end = self.trim_end().len();
        self.truncate(end);
        let start = self.len() - self.trim_start().len();
        self.drain(..start);
    }
}

pub struct CiUtils;
impl CiUtils {
    #[inline]
    pub fn bytes_eq_ci(a: &[u8], b: &[u8]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == *y)
    }

    /// Case-insensitive substring search over raw bytes — no allocation.
    /// Only handles ASCII correctly!!
    #[inline]
    pub fn bytes_contain_ci(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        if needle.len() > haystack.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| {
            w.iter()
                .zip(needle.iter())
                .all(|(h, n)| h.to_ascii_lowercase() == *n)
        })
    }

    #[inline]
    pub fn memrchr_slash(bytes: &[u8]) -> Option<usize> {
        bytes.iter().rposition(|&b| b == b'/')
    }
}
