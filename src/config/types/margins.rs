/// Window margins. Reads `[top, right, bottom, left]` or `{top = ..., ...}`. Writes the array
/// version.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl serde::Serialize for Margins {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.top, self.right, self.bottom, self.left).serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for Margins {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Touple(i32, i32, i32, i32),
            Table {
                #[serde(default)]
                top: i32,
                #[serde(default)]
                right: i32,
                #[serde(default)]
                bottom: i32,
                #[serde(default)]
                left: i32,
            },
        }
        Ok(match Repr::deserialize(d)? {
            Repr::Touple(top, right, bottom, left)
            | Repr::Table {
                top,
                right,
                bottom,
                left,
            } => Self {
                top,
                right,
                bottom,
                left,
            },
        })
    }
}
