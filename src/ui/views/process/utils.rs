use gpui::SharedString;

use crate::utils::string::CiUtils;

#[derive(Clone)]
pub struct ProcessResult {
    pub name: SharedString,
    pub pid: i32,
    pub ppid: i32,
    pub(super) score: f32,
}
impl ProcessResult {}

/// A min-heap slot: we keep a fixed-size sorted array.
/// Scores are inverted so the *worst* result is at index 0 for fast eviction.
pub struct ResultHeap {
    buf: Vec<ProcessResult>,
    capacity: usize,
}

impl ResultHeap {
    #[inline]
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Returns true if the result was inserted.
    #[inline]
    pub(super) fn push(&mut self, result: ProcessResult) -> bool {
        if self.buf.len() < self.capacity {
            self.buf.push(result);
            // Keep sorted descending (best first = lowest score first in our scheme)
            self.buf
                .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            return true;
        }
        // Only evict the worst (last, highest score) if new result is better
        if let Some(worst) = self.buf.last()
            && result.score < worst.score
        {
            *self.buf.last_mut().unwrap() = result;
            self.buf
                .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            return true;
        }

        false
    }

    pub(super) fn snapshot(&self) -> Vec<ProcessResult> {
        self.buf.clone()
    }
}

pub(super) struct PathSearchUtils;
impl PathSearchUtils {
    /// Case-insensitive substring search over raw bytes — no allocation.
    /// Only handles ASCII correctly!!
    #[inline]
    pub(super) fn bytes_contain_ci(haystack: &[u8], needle: &[u8]) -> bool {
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
    // Comparing with already lower-cased query,
    // using a zero-alloc case-insensitive comparator
    #[inline]
    pub(super) fn score_ci(name_bytes: &[u8], query: &str) -> f32 {
        let q = query.as_bytes();
        let len = name_bytes.len();
        let qlen = q.len();

        let eq = len == qlen && CiUtils::bytes_eq_ci(name_bytes, q);
        let ends = len > qlen && CiUtils::bytes_eq_ci(&name_bytes[len - qlen..], q);
        let starts = len > qlen && CiUtils::bytes_eq_ci(&name_bytes[..qlen], q);

        if eq {
            return 0.0;
        }
        if ends {
            return 0.05;
        }
        if starts {
            return 0.1 + 0.1 * (1.0 - qlen as f32 / len as f32);
        }
        if Self::bytes_contain_ci(name_bytes, q) {
            return 0.4;
        }
        0.8
    }
}
