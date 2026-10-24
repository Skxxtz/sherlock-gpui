const WORDS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PluginCapability([u64; WORDS]);

impl PluginCapability {
    pub const NONE: Self = Self([0; WORDS]);

    pub const fn from_bit(n: u64) -> Self {
        let mut words = [0u64; WORDS];
        words[(n / 64) as usize] |= 1u64 << (n % 64);
        Self(words)
    }

    #[inline]
    pub fn allows(self, other: PluginCapability) -> bool {
        let mut i = 0;
        while i < WORDS {
            if (self.0[i] & other.0[i]) != other.0[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl std::ops::BitOr for PluginCapability {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        let mut out = [0u64; WORDS];
        let mut i = 0;
        while i < WORDS {
            out[i] = self.0[i] | rhs.0[i];
            i += 1;
        }
        Self(out)
    }
}

impl std::ops::BitOrAssign for PluginCapability {
    fn bitor_assign(&mut self, rhs: Self) {
        let mut i = 0;
        while i < WORDS {
            self.0[i] |= rhs.0[i];
            i += 1;
        }
    }
}

pub trait HasCapabilityBit {
    const CAPABILITY: PluginCapability;
}
