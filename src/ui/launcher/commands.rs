use gpui::{Action, Context, Keystroke, SharedString, Window};

use crate::{
    launcher::utils::exec_mode::ExecMode,
    ui::{
        actions::{
            launcher::{
                Back,
                ErrorPage,
                Execute,
                ExecuteInplace,
                ItemDown,
                ItemLeft,
                ItemRight,
                ItemUp,
                MaximizeView,
                MinimizeView,
                NextVar,
                PrevVar,
                Quit,
                ToggleContext,
            },
            search_bar::{Backspace as SearchBackspace, ShortcutAction},
        },
        commands::{self, Command, Key, Pressed, always},
        components::search_bar,
        launcher::{
            LauncherView,
            context_menu::wrapper::Move,
            execute::{ExecRequest, ExecSource},
            navigator::views::MoveDirection,
        },
        views::Propagation,
    },
};

/// `run` for commands that always take the key.
fn done(f: impl FnOnce()) -> Propagation {
    f();
    Propagation::Stop
}

fn menu_open(l: &LauncherView, _: &gpui::App) -> bool {
    l.context_menu.is_open()
}

/// A layer command bound to one action.
const fn cmd(
    id: &'static str,
    label: &'static str,
    keys: &'static [Key],
    when: fn(&LauncherView, &gpui::App) -> bool,
    run: fn(&mut LauncherView, &mut Window, &mut Context<LauncherView>) -> Propagation,
) -> Command<LauncherView> {
    Command {
        id,
        label,
        keys,
        when,
        run,
        hint: false,
    }
}

/// The open context menu, the first layer: it keeps its keys from the view and the search bar.
/// Its action keys (letters) stay with `context_menu_binds_listener`.
const MENU: &[Command<LauncherView>] = &[
    cmd(
        "menu.up",
        "up",
        &[Key::action::<ItemUp>()],
        menu_open,
        |l, _, cx| done(|| l.move_menu(MoveDirection::Up, cx)),
    ),
    cmd(
        "menu.down",
        "down",
        &[Key::action::<ItemDown>()],
        menu_open,
        |l, _, cx| done(|| l.move_menu(MoveDirection::Down, cx)),
    ),
    cmd(
        "menu.left",
        "left",
        &[Key::action::<ItemLeft>()],
        menu_open,
        |l, _, cx| done(|| l.move_menu(MoveDirection::Left, cx)),
    ),
    cmd(
        "menu.right",
        "right",
        &[Key::action::<ItemRight>()],
        menu_open,
        |l, _, cx| done(|| l.move_menu(MoveDirection::Right, cx)),
    ),
    cmd(
        "menu.run",
        "run",
        &[Key::action::<Execute>()],
        menu_open,
        |l, win, cx| done(|| l.run_menu_action(true, win, cx)),
    ),
    cmd(
        "menu.run_inplace",
        "run in place",
        &[Key::action::<ExecuteInplace>()],
        menu_open,
        |l, win, cx| done(|| l.run_menu_action(false, win, cx)),
    ),
    cmd(
        "menu.close",
        "close",
        &[Key::action::<Quit>(), Key::action::<ToggleContext>()],
        menu_open,
        |l, _, cx| done(|| l.close_context(cx)),
    ),
];

/// `ctrl-<digit>`: the n-th result.
macro_rules! shortcuts {
    ($($n:literal),*) => {[$(
        cmd(
            concat!("launcher.run_", $n),
            concat!("run #", $n),
            &[Key::Action(|| Box::new(ShortcutAction { index: $n }))],
            always,
            |l, win, cx| done(|| l.run_selected(ExecSource::Shortcut($n), true, win, cx)),
        ),
    )*]};
}
const SHORTCUTS: [Command<LauncherView>; 9] = shortcuts!(1, 2, 3, 4, 5, 6, 7, 8, 9);

/// The launcher's defaults, the last layer: what a key does when no view or search bar command
/// took it.
pub const COMMANDS: &[Command<LauncherView>] = &[
    // ↑ ↓ ← →: the selected item moves inside itself first, otherwise the selection moves.
    cmd(
        "launcher.up",
        "up",
        &[Key::action::<ItemUp>()],
        always,
        |l, win, cx| done(|| l.move_selection(MoveDirection::Up, win, cx)),
    ),
    cmd(
        "launcher.down",
        "down",
        &[Key::action::<ItemDown>()],
        always,
        |l, win, cx| done(|| l.move_selection(MoveDirection::Down, win, cx)),
    ),
    cmd(
        "launcher.left",
        "left",
        &[Key::action::<ItemLeft>()],
        always,
        |l, win, cx| done(|| l.move_selection(MoveDirection::Left, win, cx)),
    ),
    cmd(
        "launcher.right",
        "right",
        &[Key::action::<ItemRight>()],
        always,
        |l, win, cx| done(|| l.move_selection(MoveDirection::Right, win, cx)),
    ),
    // Tab / Shift+Tab: variable inputs.
    cmd(
        "launcher.next_field",
        "next field",
        &[Key::action::<NextVar>()],
        always,
        |l, win, cx| done(|| l.focus_next_var(win, cx)),
    ),
    cmd(
        "launcher.prev_field",
        "previous field",
        &[Key::action::<PrevVar>()],
        always,
        |l, win, cx| done(|| l.focus_prev_var(win, cx)),
    ),
    // Enter / Ctrl+Enter: the selection.
    cmd(
        "launcher.run",
        "run",
        &[Key::action::<Execute>()],
        always,
        |l, win, cx| done(|| l.run_selected(ExecSource::Selected, true, win, cx)),
    ),
    cmd(
        "launcher.run_inplace",
        "run in place",
        &[Key::action::<ExecuteInplace>()],
        always,
        |l, win, cx| done(|| l.run_selected(ExecSource::Selected, false, win, cx)),
    ),
    cmd(
        "launcher.open_context",
        "actions",
        &[Key::action::<ToggleContext>()],
        always,
        |l, win, cx| done(|| l.open_context_menu(win, cx)),
    ),
    cmd(
        "launcher.messages",
        "messages",
        &[Key::action::<ErrorPage>()],
        always,
        |l, win, cx| done(|| l.show_messages(win, cx)),
    ),
    cmd(
        "launcher.quit",
        "quit",
        &[Key::action::<Quit>()],
        always,
        |l, win, cx| done(|| l.close_window(win, cx)),
    ),
    // ⌫ in an empty search bar (`Back`), or on a page without one.
    cmd(
        "launcher.back",
        "back",
        &[Key::action::<Back>()],
        always,
        |l, win, cx| done(|| l.go_back(win, cx)),
    ),
    cmd(
        "launcher.page_back",
        "back",
        &[Key::action::<SearchBackspace>()],
        |l, cx| !l.navigation.current().handle.chrome(cx).search_bar,
        |l, win, cx| done(|| l.go_back(win, cx)),
    ),
    cmd(
        "launcher.maximize",
        "restore",
        &[Key::action::<MaximizeView>()],
        always,
        |l, _, cx| done(|| l.restore_minimized(cx)),
    ),
    cmd(
        "launcher.minimize",
        "minimize",
        &[Key::action::<MinimizeView>()],
        always,
        |l, _, cx| done(|| l.minimize_view(cx)),
    ),
    SHORTCUTS[0],
    SHORTCUTS[1],
    SHORTCUTS[2],
    SHORTCUTS[3],
    SHORTCUTS[4],
    SHORTCUTS[5],
    SHORTCUTS[6],
    SHORTCUTS[7],
    SHORTCUTS[8],
];

impl LauncherView {
    /// Offers a keystroke or action to the layers: context menu → view → search bar →
    /// launcher. `true`: a command took it.
    fn command(&mut self, pressed: &Pressed, win: &mut Window, cx: &mut Context<Self>) -> bool {
        let handle = self.navigation.current().handle.clone();
        // The search bar's editing only while it has focus (not a variable input's or a page's).
        let search_focused = self.text_input.read(cx).focus_handle.is_focused(win);
        let handled = commands::dispatch(MENU, self, pressed, win, cx) == Propagation::Stop
            || handle.command(pressed, win, cx) == Propagation::Stop
            || (search_focused
                && self.text_input.update(cx, |input, cx| {
                    commands::dispatch(search_bar::commands::COMMANDS, input, pressed, win, cx)
                }) == Propagation::Stop)
            || commands::dispatch(COMMANDS, self, pressed, win, cx) == Propagation::Stop;
        if handled {
            cx.notify();
        }
        handled
    }

    /// A keystroke, before GPUI dispatches it. Only while the search bar or the launcher itself
    /// has focus: a focused component (variable input, settings control) gets its keys first,
    /// and what it passes on reaches the layers as an action (`route_action`).
    pub(super) fn command_key(
        &mut self,
        key: &Keystroke,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let search_focused = self.text_input.read(cx).focus_handle.is_focused(win);
        if !search_focused && !self.focus_handle.is_focused(win) {
            return false;
        }
        let pressed = Pressed::key(key, win, cx);
        self.command(&pressed, win, cx)
    }

    /// An action that reached the launcher without a key (status-bar click, `EmptyBackspace`,
    /// passed on by a focused component).
    pub(super) fn command_action(
        &mut self,
        action: &dyn Action,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.command(&Pressed::action(action), win, cx)
    }

    /// The launcher's listener for every bindable action: hands it to the layers.
    pub(super) fn route_action<A: Action>(
        &mut self,
        action: &A,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_action(action, win, cx);
    }

    /// Runs command `id` in the first layer that has it available. `true`: a command took it.
    pub fn run_command(&mut self, id: &str, win: &mut Window, cx: &mut Context<Self>) -> bool {
        let handle = self.navigation.current().handle.clone();
        let handled = commands::run_id(MENU, self, id, win, cx) == Propagation::Stop
            || handle.run_command(id, win, cx) == Propagation::Stop
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

// What the commands run.
impl LauncherView {
    fn move_menu(&mut self, direction: MoveDirection, cx: &mut Context<Self>) {
        self.context_menu.handle_move(direction);
        cx.notify();
    }

    fn run_menu_action(&mut self, exit: bool, win: &mut Window, cx: &mut Context<Self>) {
        if let Some(action) = self.context_menu.get_current().cloned() {
            self.execute_context_action(action, exit, win, cx);
        }
    }

    fn run_selected(
        &mut self,
        source: ExecSource,
        exit: bool,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(req) = self.request(source, exit, cx) {
            self.run(req, win, cx);
        }
    }

    fn show_messages(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        self.run(
            ExecRequest {
                mode: ExecMode::ShowMessages,
                source: ExecSource::Bind,
                query: self.text_input.read(cx).content.clone(),
                keyword: SharedString::default(),
                key: None,
                exit: false,
            },
            win,
            cx,
        );
    }

    fn restore_minimized(&mut self, cx: &mut Context<Self>) {
        if self.navigation.restore_minimized() {
            self.reset_query(cx);
        }
    }

    fn minimize_view(&mut self, cx: &mut Context<Self>) {
        if self.navigation.minimize_current(cx) {
            self.reset_query(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = MENU.iter().chain(COMMANDS).map(|c| c.id).collect();
        let len = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), len);
    }

    #[test]
    fn shortcuts_match_their_digit_only() {
        let third = Pressed::action(&ShortcutAction { index: 3 });
        let matching: Vec<_> = COMMANDS
            .iter()
            .filter(|c| c.matches(&third))
            .map(|c| c.id)
            .collect();
        assert_eq!(matching, ["launcher.run_3"]);
    }
}
