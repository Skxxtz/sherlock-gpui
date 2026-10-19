//! Deterministic RNG, so generated data is reproducible.

/// Small deterministic RNG (xorshift64), so generated data is reproducible.
pub struct Rng(u64);

impl Rng {
    /// Seeded generator; the same seed always gives the same data.
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "Rng::below(0): tried to pick from an empty list");
        (self.next_u64() % n as u64) as usize
    }

    /// `true` with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next_u64() % 1_000_000) as f64 / 1_000_000.0 < p
    }

    /// A random element.
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len())]
    }
}
