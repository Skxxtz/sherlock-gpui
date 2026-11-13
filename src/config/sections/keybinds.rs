use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Section;
use crate::{
    config::{defaults::KeybindDefaults, keys},
    ui::actions::UIFunction,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigKeybinds(pub HashMap<String, UIFunction>);

impl Section for ConfigKeybinds {}

impl ConfigKeybinds {
    /// The user's keys in GPUI's spelling, and the ones that couldn't be read.
    pub fn normalized(&self) -> (HashMap<String, UIFunction>, Vec<String>) {
        let mut binds = HashMap::new();
        let mut errors = Vec::new();
        for (key, function) in &self.0 {
            match keys::normalize(key) {
                Ok(key) => {
                    binds.insert(key, *function);
                }
                Err(e) => errors.push(format!(
                    "[keybinds] {e}: ignored, `{function:?}` keeps its default keys"
                )),
            }
        }
        (binds, errors)
    }

    /// What is bound, one action per key: the built-in keys, with the user's entries on top.
    /// A user key rebinds that key (also one of another action's built-in keys);
    /// `"key" = "unbind"` removes a built-in key.
    pub fn effective(&self) -> HashMap<String, UIFunction> {
        let (user, _) = self.normalized();
        let mut binds: HashMap<String, UIFunction> = KeybindDefaults::defaults()
            .iter()
            .map(|&(key, f)| (key.to_string(), f))
            .collect();
        for (key, function) in user {
            match function {
                UIFunction::Unbind => binds.remove(&key),
                function => binds.insert(key, function),
            };
        }
        binds
    }
}
