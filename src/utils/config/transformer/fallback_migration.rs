use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    launcher::{utils::binds::BindSerde, variant_type::LauncherVariant},
    loader::utils::{ApplicationActionSerde, ExecVariable, RawLauncher},
    utils::config::HomeType,
};

fn default_true() -> bool {
    true
}

pub struct MigrationResult {
    pub launcher: RawLauncher,
    pub logs: Vec<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct LegacyRawLauncher {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub display_name: Option<String>,
    pub on_return: Option<String>,
    pub next_content: Option<String>,
    pub r#type: String,
    pub priority: f32,

    #[serde(default = "default_true")]
    pub exit: bool,
    #[serde(default = "default_true")]
    pub shortcut: bool,
    #[serde(default = "default_true")]
    pub spawn_focus: bool,
    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: HomeType,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub binds: Option<Vec<BindSerde>>,
    #[serde(default)]
    pub actions: Option<Vec<ApplicationActionSerde>>,
    #[serde(default)]
    pub add_actions: Option<Vec<ApplicationActionSerde>>,
    #[serde(default)]
    pub variables: Option<Vec<LegacyExecVariable>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyExecVariable {
    StringInput(String),
    PasswordInput(String),
}

impl From<LegacyExecVariable> for ExecVariable {
    fn from(value: LegacyExecVariable) -> Self {
        match value {
            LegacyExecVariable::StringInput(s) => ExecVariable::String(s.into()),
            LegacyExecVariable::PasswordInput(s) => ExecVariable::Password(s.into()),
        }
    }
}

impl LegacyRawLauncher {
    pub fn migrate_args(
        &mut self,
        variant: Option<LauncherVariant>,
        logs: &mut Vec<String>,
        name: &str,
    ) {
        let Some(var) = variant else { return };

        // --- TYPE-SPECIFIC ARGS MIGRATION ---
        let args = &mut self.args;
        match var {
            LauncherVariant::Calculator | LauncherVariant::Clipboard => {
                if let Some(obj) = args.as_object_mut()
                    && let Some(caps) = obj.get_mut("capabilities").and_then(|c| c.as_array_mut())
                {
                    let old_val = json!("colors.all");
                    let new_val = json!("colors");

                    // Check if the old value exists
                    if caps.contains(&old_val) {
                        // Remove all instances of "colors.all"
                        caps.retain(|x| x != &old_val);

                        // Add the simplified "colors" if it's not already there
                        if !caps.contains(&new_val) {
                            caps.push(new_val);
                        }

                        logs.push(format!(
                            "[{}] Renamed 'colors.all' to 'colors' in capabilities.",
                            name
                        ));
                    }
                }
            }
            LauncherVariant::Commands => {
                if let Some(exec) = args.get_mut("exec").and_then(|e| e.as_str())
                    && exec.ends_with('&')
                {
                    let cleaned = exec.trim_end_matches('&').trim().to_string();
                    args["exec"] = serde_json::Value::String(cleaned);
                    logs.push(format!(
                        "[{}] Removed trailing '&' from command exec.",
                        name
                    ));
                }
            }
            LauncherVariant::Emoji => {
                if let Some(obj) = args.as_object_mut()
                    && let Some(value) = obj.remove("default_skin_color")
                {
                    obj.insert("default_skin_tone".to_string(), value);

                    logs.push(format!(
                        "[{}] Renamed 'default_skin_color' to 'default_skin_tone'.",
                        name
                    ));
                }
            }
            _ => {}
        }
    }

    pub fn migrate_type(&self) -> Option<LauncherVariant> {
        let new_type: Option<LauncherVariant> = serde_json::from_str(self.r#type.as_str()).ok();
        if new_type.is_some() {
            return new_type;
        }
        match self.r#type.as_str() {
            "app_launcher" | "apps" => Some(LauncherVariant::Apps),
            "audio_sink" | "music_player" => Some(LauncherVariant::MusicPlayer),
            "bookmarks" => Some(LauncherVariant::Bookmarks),
            "categories" | "category" => Some(LauncherVariant::Categories),
            "clipboard-execution" | "clipboard" => Some(LauncherVariant::Clipboard),
            "command" | "commands" => Some(LauncherVariant::Commands),
            "emoji_picker" | "emoji" => Some(LauncherVariant::Emoji),
            "weather" => Some(LauncherVariant::Weather),
            "web_launcher" | "web" => Some(LauncherVariant::Web),
            "calculation" | "calculator" => Some(LauncherVariant::Calculator),
            "bulk_text" | "script" => Some(LauncherVariant::Script),
            "files" => Some(LauncherVariant::Files),
            "teams_event" | "event" => Some(LauncherVariant::Event),
            "debug" => Some(LauncherVariant::Debug),
            "theme_picker" | "theme" => Some(LauncherVariant::Theme),
            "process" => Some(LauncherVariant::Process),
            "pomodoro" | "timer" => Some(LauncherVariant::Timer),
            _ => None,
        }
    }

    pub fn migrate(mut self) -> Result<MigrationResult, Vec<String>> {
        let mut logs = Vec::new();
        let name = self.name.clone().unwrap_or_else(|| "Unknown".to_string());

        // 1. Check for removed tags
        if self.tag_start.is_some() || self.tag_end.is_some() {
            logs.push(format!(
                "[{}] Removed legacy tags (tag_start/tag_end).",
                name
            ));
        }

        // Check for keybinds
        if let Some(binds) = &mut self.binds {
            for bind in binds {
                bind.bind = bind
                    .bind
                    .split("+")
                    .map(|token| match token {
                        "control" => "ctrl",
                        "lock" => "lock",
                        "hypr" => "cmd",
                        "meta" => "win",
                        _ => token,
                    })
                    .collect::<Vec<_>>()
                    .join("-")
                    .to_lowercase();
            }
        }

        // 3. Log variable conversion
        if let Some(ref vars) = self.variables {
            logs.push(format!(
                "[{}] Migrated {} variables to new format.",
                name,
                vars.len()
            ));
        }

        let new_type = self.migrate_type();

        self.migrate_args(new_type, &mut logs, &name);

        let Some(new_type) = self.migrate_type() else {
            logs.push(format!("[{name}] Failed to parse type. ({})", self.r#type));
            return Err(logs);
        };

        // drop async for all other types execpt script, because its automatic
        if new_type == LauncherVariant::Script
            && let Some(obj) = self.args.as_object_mut()
        {
            logs.push("[{name}] Moved `async` field into `args`".into());
            obj.insert("async".to_string(), json!(self.r#async));
        }

        // notify for async drop for all other types
        if self.r#async {
            logs.push("[{name}] Removed `async` field. It's not required anymore".into());
        }

        // move display_name into args
        if let Some(dis) = &self.display_name
            && let Some(obj) = self.args.as_object_mut()
        {
            logs.push("[{name}] Moved `display_name` field into `args`".into());
            obj.insert("display_name".to_string(), json!(dis));
        }

        let launcher = RawLauncher {
            name: self.name,
            alias: self.alias,
            on_return: self.on_return,
            next_content: self.next_content,
            r#type: new_type,
            priority: self.priority as u16,
            limit: None,
            exit: self.exit,
            shortcut: self.shortcut,
            spawn_focus: self.spawn_focus,
            home: self.home,
            sidebar_default_on: false,
            binds: self.binds,
            args: Arc::new(self.args),
            actions: self.actions,
            add_actions: self.add_actions,
            variables: self
                .variables
                .map(|vars| vars.into_iter().map(ExecVariable::from).collect()),
        };

        Ok(MigrationResult { launcher, logs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_legacy_to_modern_migration() {
        // 1. Create Legacy JSON (Simulating an old config file)
        let legacy_json = json!({
            "name": "Test Launcher",
            "type": "command",
            "priority": 1.0,
            "tag_start": "old_tag", // This should be dropped/logged
            "args": { "cmd": "ls" },
            "variables": [
                { "string_input": "username" }
            ],
            "binds": [
                {
                    "bind": "Return",
                    "callback": "playpause",
                    "exit": false
                },
                {
                    "bind": "control+l",
                    "callback": "next",
                    "exit": false
                },
                {
                    "bind": "control+h",
                    "callback": "previous",
                    "exit": false
                }
            ],
        });

        // 2. Deserialize into Legacy struct
        let legacy: LegacyRawLauncher =
            serde_json::from_value(legacy_json).expect("Failed to parse legacy JSON");

        // 3. Perform Migration
        let result = legacy.migrate().expect("Failed to parse migrate launcher");

        // 4. Assertions
        assert_eq!(result.launcher.name, Some("Test Launcher".to_string()));
        assert_eq!(result.launcher.r#type, LauncherVariant::Commands);

        // Check if Arc wrapping worked
        assert_eq!(result.launcher.args.get("cmd").unwrap(), "ls");

        if let Some(binds) = result.launcher.binds {
            assert_eq!(binds.len(), 3);
            let is = binds.iter().map(|b| b.bind.as_str()).collect::<Vec<&str>>();
            let should_be: Vec<&str> = vec!["return", "ctrl-l", "ctrl-h"];
            for (is, sb) in is.iter().zip(should_be) {
                assert_eq!(is, &sb);
            }
        } else {
            panic!("Variables were lost during migration!");
        }

        // Verify Variable migration
        if let Some(vars) = result.launcher.variables {
            assert_eq!(vars.len(), 1);
            // Ensure the inner enum converted correctly
            match &vars[0] {
                ExecVariable::String(s) => assert_eq!(s, "username"),
                _ => panic!("Variable type mismatch!"),
            }
        } else {
            panic!("Variables were lost during migration!");
        }

        println!("{:?}", result.logs.join("\n"));

        // Verify Logs caught the missing fields
        assert!(
            result
                .logs
                .iter()
                .any(|l| l.contains("Removed legacy tags"))
        );
    }
}
