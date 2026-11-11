use gpui::{Context, Keystroke, Window};

use crate::ui::{
    commands::{self, Command},
    components::search_bar,
    launcher::LauncherView,
    views::Propagation,
};

/// The launcher's layer, the last one. Its defaults still run through GPUI actions for now.
pub const COMMANDS: &[Command<LauncherView>] = &[];

impl LauncherView {
    /// Offers `key` to the layers: view → search bar → launcher. `true`: a command took it.
    pub(super) fn command_key(
        &mut self,
        key: &Keystroke,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let handle = self.navigation.current().handle.clone();
        let handled = handle.command_key(key, win, cx) == Propagation::Stop
            || self.text_input.update(cx, |input, cx| {
                commands::dispatch_key(search_bar::commands::COMMANDS, input, key, win, cx)
            }) == Propagation::Stop
            || commands::dispatch_key(COMMANDS, self, key, win, cx) == Propagation::Stop;
        if handled {
            cx.notify();
        }
        handled
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
