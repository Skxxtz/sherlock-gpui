use crate::ui::launcher::navigator::views::MoveDirection;

/// A launcher-level input, already resolved from the keybinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherEvent {
    /// Arrow keys / item binds. Default: move the selection (or inside the selected item).
    Move(MoveDirection),
    /// Enter (`exit: true`) or run in place (`exit: false`). Default: run the selected item.
    Execute { exit: bool },
    /// Default: toggle the context menu of the selected item.
    OpenContext,
    /// Backspace on an empty search bar, or on a page without one. Default: previous view, or
    /// leave the current mode.
    Back,
    /// Escape. Default: close the launcher.
    Quit,
}

/// Whether the shell still runs its default after the view saw an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Propagation {
    /// The view handled it; skip the default.
    Stop,
    /// Run the default.
    #[default]
    Continue,
}
