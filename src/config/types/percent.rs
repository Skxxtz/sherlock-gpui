/// A fraction in `0.0..=1.0`. Reads `0.6` and `60` alike.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Percent(f32);

impl Percent {
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
    pub fn get(self) -> f32 {
        self.0
    }
    pub fn get_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl serde::Serialize for Percent {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // rounded, so 0.6 is not written as 0.6000000238418579
        s.serialize_f64((self.0 as f64 * 1000.).round() / 1000.)
    }
}

impl<'de> serde::Deserialize<'de> for Percent {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f64::deserialize(d)?;
        let v = if v >= 1. { v / 100. } else { v };
        Ok(Self(v.clamp(0., 1.) as f32))
    }
}
