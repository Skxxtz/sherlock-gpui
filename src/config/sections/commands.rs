use std::{collections::HashMap, sync::Arc};

use gpui::Keystroke;
use serde::{Deserialize, Serialize};

use super::Section;
use crate::{config::keys, launcher::utils::binds::Bind};

/// `[commands]`: keys to command chains (see `ui::commands`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigCommands(pub HashMap<String, String>);

impl Section for ConfigCommands {
    fn validate(&mut self, warn: &mut Vec<String>) {
        warn.extend(self.binds().1);
    }
}

impl ConfigCommands {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The binds, and the entries that couldn't be read (with why). One keystroke per entry,
    /// no context (`"variable.tab"`): commands pick their layer themselves.
    pub fn binds(&self) -> (Vec<Bind>, Vec<String>) {
        let mut binds = Vec::new();
        let mut errors = Vec::new();
        for (key, chain) in &self.0 {
            let stroke = keys::normalize(key).and_then(|k| {
                if k.contains(' ') || crate::app::bindings::split_scope(&k).0.is_some() {
                    return Err(format!("`{key}`: one keystroke without a context"));
                }
                Keystroke::parse(&k).map_err(|e| format!("`{key}`: {e}"))
            });
            match stroke {
                Ok(bind) => binds.push(Bind {
                    exit: false,
                    bind,
                    callback: Arc::new(chain.clone()),
                }),
                Err(e) => errors.push(format!("[commands] {e}: ignored")),
            }
        }
        (binds, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigCommands;

    #[test]
    fn reads_single_keystrokes_and_reports_the_rest() {
        let commands = ConfigCommands(
            [
                ("Ctrl-N", "ask.new_chat"),
                ("ctrl-k ctrl-c", "search.copy"),
                ("variable.tab", "search.complete"),
                ("ctrl-nope", "x"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into(),
        );
        let (binds, errors) = commands.binds();
        assert_eq!(binds.len(), 1);
        assert_eq!(binds[0].bind.key, "n");
        assert!(binds[0].bind.modifiers.control);
        assert_eq!(binds[0].callback.as_str(), "ask.new_chat");
        assert_eq!(errors.len(), 3, "{errors:?}");
    }
}
