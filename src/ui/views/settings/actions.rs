use gpui::Context;

use super::SettingsView;
use crate::ui::{
    actions::*,
    commands::{Command, Key, always},
    views::Propagation,
};

/// The action was handled, and will not be propagated.
fn handled(cx: &mut Context<SettingsView>) -> Propagation {
    cx.notify();
    Propagation::Stop
}

/// switch sections, unless the search bar has text (then they move the cursor).
fn browsing(v: &SettingsView, _: &gpui::App) -> bool {
    !v.searching()
}

impl SettingsView {
    pub(super) const COMMANDS: &[Command<Self>] = &[
        Command {
            id: "settings.up",
            label: "up",
            keys: &[Key::action::<ItemUp>()],
            when: always,
            run: |v, _, cx| {
                v.select_row(v.row.saturating_sub(1));
                handled(cx)
            },
            hint: false,
        },
        Command {
            id: "settings.down",
            label: "down",
            keys: &[Key::action::<ItemDown>()],
            when: always,
            run: |v, _, cx| {
                v.select_row(v.row + 1);
                handled(cx)
            },
            hint: false,
        },
        Command {
            id: "settings.prev_section",
            label: "previous section",
            keys: &[Key::action::<ItemLeft>()],
            when: browsing,
            run: |v, _, cx| {
                v.select_section(v.active.saturating_sub(1));
                handled(cx)
            },
            hint: false,
        },
        Command {
            id: "settings.next_section",
            label: "next section",
            keys: &[Key::action::<ItemRight>()],
            when: browsing,
            run: |v, _, cx| {
                v.select_section(v.active + 1);
                handled(cx)
            },
            hint: false,
        },
        Command {
            id: "settings.edit",
            label: "edit",
            keys: &[Key::action::<Execute>(), Key::action::<ExecuteInplace>()],
            when: always,
            run: |v, win, cx| {
                v.activate(win, cx);
                handled(cx)
            },
            hint: false,
        },
    ];
}
