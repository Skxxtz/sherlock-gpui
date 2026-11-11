use gpui::{Context, Window};

use super::{Command, always};
use crate::ui::{
    actions::*,
    traits::{HandleActions, InputAction},
    views::Propagation,
};

fn route<V: HandleActions, A: InputAction + Default>(
    this: &mut V,
    win: &mut Window,
    cx: &mut Context<V>,
) -> Propagation {
    A::default().route(this, win, cx)
}

macro_rules! action_commands {
    ($($action:ty => $id:literal, $label:literal;)*) => {
        /// `HandleActions` methods as commands (`Self::ACTION_COMMANDS`).
        pub trait ActionCommands: HandleActions {
            const ACTION_COMMANDS: &'static [Command<Self>] = &[$(
                Command {
                    id: $id,
                    label: $label,
                    keys: &[],
                    when: always::<Self>,
                    run: route::<Self, $action>,
                    hint: false,
                },
            )*];
        }

        impl<V: HandleActions> ActionCommands for V {}
    };
}

action_commands! {
    ItemUp => "view.item_up", "Up";
    ItemDown => "view.item_down", "Down";
    ItemLeft => "view.item_left", "Left";
    ItemRight => "view.item_right", "Right";
    Execute => "view.execute", "Run";
    ExecuteInplace => "view.execute_inplace", "Run in place";
    Back => "view.back", "Back";
    Quit => "view.quit", "Quit";
    NextVar => "view.next_var", "Next field";
    PrevVar => "view.prev_var", "Previous field";
    ToggleContext => "view.open_context", "Actions";
    Complete => "view.complete", "Complete";
    Backspace => "view.backspace", "Delete";
    DeleteForward => "view.delete_forward", "Delete forward";
    DeleteAll => "view.delete_all", "Clear";
    SelectAll => "view.select_all", "Select all";
    Home => "view.home", "Start";
    End => "view.end", "End";
    Paste => "view.paste", "Paste";
    Copy => "view.copy", "Copy";
    Cut => "view.cut", "Cut";
}
