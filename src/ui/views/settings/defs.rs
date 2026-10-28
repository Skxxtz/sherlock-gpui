use gpui::SharedString;

use super::{
    config_value::Meta,
    model::{Setting, SettingValue, SettingsSection},
};
use crate::{
    app::bindings::split_scope,
    ui::actions::UIFunction,
    utils::config::{
        ConfigAppearance,
        ConfigBackdrop,
        ConfigBehavior,
        ConfigCaching,
        ConfigDebug,
        ConfigDefaultApps,
        KeybindDefaults,
        SearchBarIcon,
        SherlockConfig,
        StatusBar,
    },
};

/// One row: where its value lives in the config and how it's shown. Made by
/// `#[derive(Settings)]` on the config structs, and by `keybind!` below.
pub(crate) struct SettingDef {
    /// The config key path, e.g. `"behavior.animate"`; also the setting's id.
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub meta: Meta,
    pub get: fn(&SherlockConfig, Meta) -> SettingValue,
    pub set: fn(&mut SherlockConfig, &SettingValue),
}

pub(super) struct SectionDef {
    pub title: &'static str,
    pub description: &'static str,
    pub settings: Vec<SettingDef>,
}

impl SettingDef {
    pub(super) fn read(&self, config: &SherlockConfig) -> Setting {
        Setting {
            id: self.id,
            group: self.group,
            label: self.label,
            description: self.description,
            value: (self.get)(config, self.meta),
        }
    }
}

impl SectionDef {
    /// A section whose rows are kept together by group (in order of each group's first row),
    /// so field order in the structs doesn't split a group into two headings.
    fn new(title: &'static str, description: &'static str, settings: Vec<SettingDef>) -> Self {
        let mut order: Vec<&'static str> = Vec::new();
        for def in &settings {
            if !order.contains(&def.group) {
                order.push(def.group);
            }
        }
        let mut settings = settings;
        settings.sort_by_key(|d| order.iter().position(|&g| g == d.group));
        Self {
            title,
            description,
            settings,
        }
    }

    pub(super) fn read(&self, config: &SherlockConfig) -> SettingsSection {
        SettingsSection {
            title: self.title,
            description: self.description,
            settings: self.settings.iter().map(|d| d.read(config)).collect(),
        }
    }
}

/// The keys of `function` (in context `"ctx"` with `in "ctx"`, else without a context), as
/// bound: the user's, or the built-in ones if the user didn't change them. `name` is the
/// action's name in `[keybinds]` (`item_down`).
macro_rules! keybind {
    ($group:literal, $name:ident : $function:ident, $label:literal, $desc:literal $(,)?) => {
        keybind!(@def $group, concat!("keybinds.", stringify!($name)), $function, None, $label, $desc)
    };
    ($group:literal, $name:ident : $function:ident in $scope:literal, $label:literal, $desc:literal $(,)?) => {
        keybind!(@def $group, concat!("keybinds.", $scope, ".", stringify!($name)), $function, Some($scope), $label, $desc)
    };
    (@def $group:literal, $id:expr, $function:ident, $scope:expr, $label:literal, $desc:literal) => {
        SettingDef {
            id: $id,
            group: $group,
            label: $label,
            description: $desc,
            meta: Meta::None,
            get: |c, _| SettingValue::Keybind(bound_chords(c, UIFunction::$function, $scope)),
            set: |c, value| set_keybind_chords(c, UIFunction::$function, $scope, value),
        }
    };
}

/// The keys of `function` in `scope` among `binds`, sorted.
fn chords_in<'a>(
    binds: impl IntoIterator<Item = (&'a str, UIFunction)>,
    function: UIFunction,
    scope: Option<&str>,
) -> Vec<SharedString> {
    let mut chords: Vec<SharedString> = binds
        .into_iter()
        .filter(|&(_, f)| f == function)
        .filter_map(|(key, _)| match split_scope(key) {
            (s, chord) if s == scope => Some(SharedString::from(chord.to_string())),
            _ => None,
        })
        .collect();
    chords.sort();
    chords
}

/// What `function` is bound to now in `scope`.
fn bound_chords(
    config: &SherlockConfig,
    function: UIFunction,
    scope: Option<&str>,
) -> Vec<SharedString> {
    let binds = config.keybinds.effective();
    chords_in(binds.iter().map(|(k, f)| (k.as_str(), *f)), function, scope)
}

/// The built-in keys of `function` in `scope`, as written in the file (`"variable.tab"`).
fn default_keys(function: UIFunction, scope: Option<&str>) -> Vec<String> {
    KeybindDefaults::defaults()
        .iter()
        .filter(|&&(key, f)| f == function && split_scope(key).0 == scope)
        .map(|&(key, _)| key.to_string())
        .collect()
}

/// Sets `function`'s keys in `scope`, written as the difference from the built-in keys. A key
/// runs one action: one taken from another action (built-in or not) moves here.
fn set_keybind_chords(
    config: &mut SherlockConfig,
    function: UIFunction,
    scope: Option<&str>,
    value: &SettingValue,
) {
    let SettingValue::Keybind(chords) = value else {
        return;
    };
    let wanted: Vec<String> = chords
        .iter()
        .map(|chord| match scope {
            Some(scope) => format!("{scope}.{chord}"),
            None => chord.to_string(),
        })
        .collect();
    let defaults = default_keys(function, scope);
    let binds = &mut config.keybinds.0;

    // drop what the file said about this action's keys in `scope`: its own entries, and
    // `unbind`s of its built-in keys; then write the difference again
    binds.retain(|key, f| {
        let ours = *f == function && split_scope(key).0 == scope;
        let our_unbind = *f == UIFunction::Unbind && defaults.contains(key);
        !ours && !our_unbind
    });

    for key in &wanted {
        // a built-in key of ours that nothing else claims needs no entry; anything else is
        // written, which also takes the key from whoever has it
        // TODO: ask before moving a key from another action (a popup: "ctrl enter runs Run in
        // place — move it here?"), instead of moving it silently.
        if !(defaults.contains(key) && !binds.contains_key(key)) {
            binds.insert(key.clone(), function);
        }
    }
    for key in &defaults {
        // a built-in key we no longer want: unbind it, unless another action took it (its
        // entry already overrides the default)
        if !wanted.contains(key) && !binds.contains_key(key) {
            binds.insert(key.clone(), UIFunction::Unbind);
        }
    }
}

/// Everything the settings page shows, in order.
pub(super) fn sections() -> Vec<SectionDef> {
    vec![
        SectionDef::new(
            "General",
            "How Sherlock behaves while you search and run things.",
            [
                ConfigBehavior::setting_defs(),
                ConfigCaching::setting_defs(),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        SectionDef::new(
            "Appearance",
            "Size, look and the parts of the window. Applied the next time it opens.",
            [
                ConfigAppearance::setting_defs(),
                SearchBarIcon::setting_defs(),
                StatusBar::setting_defs(),
                ConfigBackdrop::setting_defs(),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        SectionDef::new(
            "Apps",
            "Programs Sherlock opens for you.",
            ConfigDefaultApps::setting_defs(),
        ),
        SectionDef::new(
            "Keyboard",
            "The keys for every action. Change one to replace its built-in keys.",
            vec![
                keybind!("navigation", item_up: ItemUp, "Previous item", "Move the selection up."),
                keybind!("navigation", item_down: ItemDown, "Next item", "Move the selection down."),
                keybind!("navigation", item_left: ItemLeft, "Item left", "Move left in grids, or to the previous section."),
                keybind!("navigation", item_right: ItemRight, "Item right", "Move right in grids, or to the next section."),
                keybind!("navigation", back: Back, "Back", "Go back to the previous view."),
                keybind!("actions", execute: Execute, "Run", "Run the selected item."),
                keybind!("actions", execute_inplace: ExecuteInplace, "Run in place", "Run without closing the launcher."),
                keybind!("actions", toggle_context: ToggleContext, "Context menu", "Open actions for the selected item."),
                keybind!("actions", quit: Quit, "Quit", "Close the launcher."),
                keybind!("arguments", next_var: NextVar, "Next argument", "Move to the next argument field."),
                keybind!("arguments", prev_var: PrevVar, "Previous argument", "Move to the previous argument field."),
                keybind!("arguments", complete: Complete in "variable", "Complete argument", "Complete a runtime variable if possible."),
                keybind!("editing", backspace: Backspace, "Delete back", "Delete the character before the caret."),
                keybind!("editing", delete_forward: DeleteForward, "Delete forward", "Delete the character after the caret."),
                keybind!("editing", delete_all: DeleteAll, "Clear", "Empty the search bar."),
                keybind!("editing", select_all: SelectAll, "Select all", "Select the whole search text."),
                keybind!("editing", cursor_left: CursorLeft, "Caret left", "Move the caret one character left."),
                keybind!("editing", cursor_right: CursorRight, "Caret right", "Move the caret one character right."),
                keybind!("editing", home: Home, "Start of line", "Move the caret to the start."),
                keybind!("editing", end: End, "End of line", "Move the caret to the end."),
                keybind!("clipboard", copy: Copy, "Copy", "Copy the selected text."),
                keybind!("clipboard", cut: Cut, "Cut", "Cut the selected text."),
                keybind!("clipboard", paste: Paste, "Paste", "Paste from the clipboard."),
            ],
        ),
        SectionDef::new("Advanced", "Troubleshooting.", ConfigDebug::setting_defs()),
    ]
}
