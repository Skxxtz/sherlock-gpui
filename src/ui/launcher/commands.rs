use gpui::{Action, Context, Keystroke, Window};

use crate::ui::{
    commands::{self, Command, Pressed},
    components::search_bar,
    launcher::LauncherView,
    views::Propagation,
};

/// The launcher's layer, the last one. Its defaults still run through GPUI actions for now.
pub const COMMANDS: &[Command<LauncherView>] = &[];

impl LauncherView {
    /// Offers a keystroke or action to the layers: view → search bar → launcher. `true`: a
    /// command took it.
    fn command(&mut self, pressed: &Pressed, win: &mut Window, cx: &mut Context<Self>) -> bool {
        let handle = self.navigation.current().handle.clone();
        let handled = handle.command(pressed, win, cx) == Propagation::Stop
            || self.text_input.update(cx, |input, cx| {
                commands::dispatch(search_bar::commands::COMMANDS, input, pressed, win, cx)
            }) == Propagation::Stop
            || commands::dispatch(COMMANDS, self, pressed, win, cx) == Propagation::Stop;
        if handled {
            cx.notify();
        }
        handled
    }

    /// A keystroke, before GPUI dispatches it. An open context menu keeps its keys.
    pub(super) fn command_key(
        &mut self,
        key: &Keystroke,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.context_menu.is_open() {
            return false;
        }
        let pressed = Pressed::key(key, win, cx);
        self.command(&pressed, win, cx)
    }

    /// An action that reached the launcher without a key (status-bar click, `EmptyBackspace`).
    pub(super) fn command_action(
        &mut self,
        action: &dyn Action,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.command(&Pressed::action(action), win, cx)
    }

    /// Runs command `id` in the first layer that has it available. `true`: a command took it.
    pub fn run_command(&mut self, id: &str, win: &mut Window, cx: &mut Context<Self>) -> bool {
        let handle = self.navigation.current().handle.clone();
        let handled = handle.run_command(id, win, cx) == Propagation::Stop
            || self.text_input.update(cx, |input, cx| {
                commands::run_id(search_bar::commands::COMMANDS, input, id, win, cx)
            }) == Propagation::Stop
            || commands::run_id(COMMANDS, self, id, win, cx) == Propagation::Stop;
        if handled {
            cx.notify();
        }
        handled
    }

    /// Runs a chain like `"ask.stop;search.copy"`, each step as `Self::run_command`. `true`:
    /// any step was taken.
    pub fn run_commands(&mut self, chain: &str, win: &mut Window, cx: &mut Context<Self>) -> bool {
        chain
            .split(';')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .fold(false, |any, id| self.run_command(id, win, cx) | any)
    }
}
