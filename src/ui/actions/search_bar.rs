use gpui::{Action, actions};

actions!(
    text_input,
    [
        Backspace,
        Complete,
        DeleteForward,
        DeleteAll,
        CursorLeft,
        CursorRight,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
    ]
);

#[derive(Clone, PartialEq, Default, Debug, serde::Deserialize)]
pub struct ShortcutAction {
    pub index: usize,
}
impl Action for ShortcutAction {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }

    fn name_for_type() -> &'static str {
        "ShortcutAction"
    }

    fn name(&self) -> &'static str {
        "ShortcutAction"
    }

    fn partial_eq(&self, action: &dyn Action) -> bool {
        // We use downcast_ref to check if the other action is the same type
        action
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| other.index == self.index)
    }

    fn build(value: serde_json::Value) -> gpui::Result<Box<dyn Action>> {
        // This allows GPUI to create this action from a JSON/TOML value
        // e.g. { "index": 1 }
        let action: Self = serde_json::from_value(value)?;
        Ok(Box::new(action))
    }
}
