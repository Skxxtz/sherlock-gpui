use gpui::{SharedString, rgb};

use super::model::{Setting, SettingValue, SettingsSection};

fn setting(
    group: &'static str,
    label: &'static str,
    description: &'static str,
    value: SettingValue,
) -> Setting {
    Setting {
        group,
        label,
        description,
        value,
    }
}

/// An action bound to `chords` (`["ctrl-n", "down"]`).
fn keys(chords: &[&'static str]) -> SettingValue {
    SettingValue::Keybind(chords.iter().map(|&c| SharedString::from(c)).collect())
}

/// Placeholder data until settings are read from the config.
pub fn dummy_sections() -> Vec<SettingsSection> {
    use SettingValue::*;
    vec![
        SettingsSection {
            title: "General",
            description: "How Sherlock starts, and what it does once you've found something.",
            settings: vec![
                setting(
                    "startup",
                    "Launch on startup",
                    "Start the Sherlock daemon when you log in.",
                    Toggle(true),
                ),
                setting(
                    "startup",
                    "Close after run",
                    "Hide the launcher after an item was executed.",
                    Toggle(true),
                ),
                setting(
                    "behaviour",
                    "Clicks to run",
                    "How many clicks run an item; fewer just select it.",
                    Number {
                        value: 2,
                        min: 1,
                        max: 3,
                        step: 1,
                    },
                ),
            ],
        },
        SettingsSection {
            title: "Appearance",
            description: "Colors and density. Applied the next time the launcher opens.",
            settings: vec![
                setting(
                    "theme",
                    "Theme",
                    "Color scheme of the launcher.",
                    Choice {
                        options: &["System", "Dark", "Light"],
                        selected: 0,
                    },
                ),
                setting(
                    "theme",
                    "Accent color",
                    "Used for selections and highlights.",
                    Color(rgb(0xf5a524).into()),
                ),
                setting(
                    "theme",
                    "Background",
                    "Window background.",
                    Color(rgb(0x0e0e12).into()),
                ),
                setting(
                    "results",
                    "Shortcut count",
                    "How many results get a numbered shortcut.",
                    Number {
                        value: 5,
                        min: 0,
                        max: 9,
                        step: 1,
                    },
                ),
            ],
        },
        SettingsSection {
            title: "Keyboard",
            description: "Every action has a key. Click one to record a new combination.",
            settings: vec![
                setting(
                    "navigation",
                    "Next item",
                    "Move the selection down.",
                    keys(&["ctrl-n", "down"]),
                ),
                setting(
                    "navigation",
                    "Previous item",
                    "Move the selection up.",
                    keys(&["ctrl-p", "up"]),
                ),
                setting(
                    "navigation",
                    "Context menu",
                    "Open actions for the selected item.",
                    keys(&["ctrl-l"]),
                ),
                setting("window", "Quit", "Close the launcher.", keys(&["escape"])),
            ],
        },
        SettingsSection {
            title: "Search",
            description: "How results are matched and ranked. Picks stay on this device.",
            settings: vec![
                setting(
                    "ranking",
                    "Learn from picks",
                    "Rank items you pick often higher for the same query.",
                    Toggle(true),
                ),
                setting(
                    "ranking",
                    "Fuzzy matching",
                    "Also match slightly misspelled queries.",
                    Toggle(false),
                ),
            ],
        },
    ]
}
