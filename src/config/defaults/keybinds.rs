use crate::ui::actions::UIFunction;

pub struct KeybindDefaults;
impl KeybindDefaults {
    /// The built-in keys, as they would be written in `[keybinds]`. A context goes before the
    /// key (`"variable.tab"`).
    pub fn defaults() -> &'static [(&'static str, UIFunction)] {
        use UIFunction::*;
        &[
            // navigation
            ("up", ItemUp),
            ("down", ItemDown),
            ("left", ItemLeft),
            ("right", ItemRight),
            ("search_bar.left", CursorLeft),
            ("search_bar.right", CursorRight),
            ("enter", Execute),
            ("ctrl-enter", ExecuteInplace),
            ("escape", Quit),
            ("ctrl-l", ToggleContext),
            ("ctrl-<digit>", Shortcut),
            // arguments
            ("tab", NextVar),
            ("shift-tab", PrevVar),
            ("variable.tab", Complete),
            // editing
            ("backspace", Backspace),
            ("delete", DeleteForward),
            ("ctrl-backspace", DeleteAll),
            ("ctrl-a", SelectAll),
            ("ctrl-c", Copy),
            ("ctrl-x", Cut),
            ("ctrl-v", Paste),
            ("home", Home),
            ("end", End),
        ]
    }
}
