use std::sync::Arc;

use gpui::{InvalidKeystrokeError, Keystroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct Bind {
    pub exit: bool,
    pub bind: Keystroke,
    pub callback: Arc<String>,
}
impl Bind {
    #[inline]
    pub fn matches(&self, stroke: &Keystroke) -> bool {
        self.bind.modifiers == stroke.modifiers && self.bind.key == stroke.key
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BindSerde {
    pub bind: String,
    callback: Arc<String>,
    #[serde(default)]
    pub exit: bool,
}

impl TryFrom<BindSerde> for Bind {
    type Error = InvalidKeystrokeError;
    fn try_from(value: BindSerde) -> Result<Self, Self::Error> {
        Ok(Bind {
            bind: Keystroke::parse(&value.bind)?,
            callback: value.callback,
            exit: value.exit,
        })
    }
}
impl TryFrom<&BindSerde> for Bind {
    type Error = InvalidKeystrokeError;
    fn try_from(value: &BindSerde) -> Result<Self, Self::Error> {
        Ok(Bind {
            bind: Keystroke::parse(&value.bind)?,
            callback: value.callback.clone(),
            exit: value.exit,
        })
    }
}
