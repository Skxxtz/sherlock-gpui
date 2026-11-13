use std::{borrow::Borrow, ops::Deref, sync::Mutex};

use gpui::SharedString;
use serde::{Deserialize, Deserializer, Serialize};

/// A cheap-to-clone string (`SharedString`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Text(SharedString);

impl Text {
    pub const fn new_static(s: &'static str) -> Self {
        Self(SharedString::new_static(s))
    }

    pub fn shared(&self) -> SharedString {
        self.0.clone()
    }
}

impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl From<&'static str> for Text {
    fn from(s: &'static str) -> Self {
        Self(s.into())
    }
}
impl From<String> for Text {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}
impl From<SharedString> for Text {
    fn from(s: SharedString) -> Self {
        Self(s)
    }
}
impl From<Text> for SharedString {
    fn from(t: Text) -> Self {
        t.0
    }
}

/// For `Option<Text>` fields: an empty or blank string is treated as "not set".
pub fn non_empty<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Text>, D::Error> {
    let s = Option::<String>::deserialize(d)?;
    Ok(s.filter(|s| !s.trim().is_empty()).map(Text::from))
}
