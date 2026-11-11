use crate::ui::{
    actions::search_bar::{
        Backspace,
        Complete,
        Copy,
        CursorLeft,
        CursorRight,
        Cut,
        DeleteAll,
        DeleteForward,
        End,
        Home,
        Paste,
        SelectAll,
        SelectLeft,
        SelectRight,
    },
    commands::{Command, Key, always},
    components::search_bar::TextInput,
    views::Propagation,
};

/// A command for one key, always available.
const fn edit(
    id: &'static str,
    label: &'static str,
    keys: &'static [Key],
    run: fn(&mut TextInput, &mut gpui::Window, &mut gpui::Context<TextInput>) -> Propagation,
) -> Command<TextInput> {
    Command {
        id,
        label,
        keys,
        when: always,
        run,
        hint: false,
    }
}

pub const COMMANDS: &[Command<TextInput>] = &[
    // `Continue` in an empty bar: the input emits `EmptyBackspace`, the launcher's `Back`.
    edit(
        "search.delete_back",
        "delete",
        &[Key::action::<Backspace>()],
        |t, win, cx| t.delete_back(win, cx),
    ),
    edit(
        "search.delete_forward",
        "delete forward",
        &[Key::action::<DeleteForward>()],
        |t, win, cx| t.delete_forward(win, cx),
    ),
    edit(
        "search.clear",
        "clear",
        &[Key::action::<DeleteAll>()],
        |t, _, cx| t.clear(cx),
    ),
    // `Continue` at the ends: the launcher's ← / → (e.g. grid navigation).
    edit(
        "search.left",
        "left",
        &[Key::action::<CursorLeft>()],
        |t, _, cx| t.cursor_left(cx),
    ),
    edit(
        "search.right",
        "right",
        &[Key::action::<CursorRight>()],
        |t, _, cx| t.cursor_right(cx),
    ),
    edit(
        "search.select_left",
        "select left",
        &[Key::action::<SelectLeft>()],
        |t, _, cx| t.extend_left(cx),
    ),
    edit(
        "search.select_right",
        "select right",
        &[Key::action::<SelectRight>()],
        |t, _, cx| t.extend_right(cx),
    ),
    edit(
        "search.select_all",
        "select all",
        &[Key::action::<SelectAll>()],
        |t, _, cx| {
            t.select_all(cx);
            Propagation::Stop
        },
    ),
    edit(
        "search.home",
        "start",
        &[Key::action::<Home>()],
        |t, _, cx| t.move_home(cx),
    ),
    edit("search.end", "end", &[Key::action::<End>()], |t, _, cx| {
        t.move_end(cx)
    }),
    // `Continue` without ghost text.
    edit(
        "search.complete",
        "complete",
        &[Key::action::<Complete>()],
        |t, _, cx| t.accept_ghost(cx),
    ),
    edit(
        "search.paste",
        "paste",
        &[Key::action::<Paste>()],
        |t, win, cx| t.paste_clipboard(win, cx),
    ),
    // `Continue` without a selection: the launcher's copy.
    edit(
        "search.copy",
        "copy",
        &[Key::action::<Copy>()],
        |t, _, cx| t.copy_selection(cx),
    ),
    edit(
        "search.cut",
        "cut",
        &[Key::action::<Cut>()],
        |t, win, cx| t.cut_selection(win, cx),
    ),
];

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext,
        Context,
        IntoElement,
        Render,
        TestAppContext,
        VisualTestContext,
        Window,
        div,
    };

    use super::*;
    use crate::ui::{commands::Pressed, components::search_bar::TextInput};

    struct Root;
    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn edits_and_passes_on_at_the_edges(cx: &mut TestAppContext) {
        let (_root, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, _| Root);
        let input = cx.update(|_, cx| cx.new(|cx| TextInput::builder().content("ab").build(cx)));
        let press = |action: &dyn gpui::Action, cx: &mut VisualTestContext| {
            cx.update(|win, cx| {
                input.update(cx, |t, cx| {
                    crate::ui::commands::dispatch(COMMANDS, t, &Pressed::action(action), win, cx)
                })
            })
        };

        // the builder leaves the cursor at 0: → moves, ← at the start passes on
        assert_eq!(press(&CursorLeft, cx), Propagation::Continue);
        assert_eq!(press(&End, cx), Propagation::Stop);
        assert_eq!(press(&CursorRight, cx), Propagation::Continue);
        assert_eq!(press(&Backspace, cx), Propagation::Stop);
        assert_eq!(cx.update(|_, cx| input.read(cx).content.clone()), "a");
        assert_eq!(press(&Copy, cx), Propagation::Continue, "nothing selected");
        assert_eq!(press(&DeleteAll, cx), Propagation::Stop);
        assert_eq!(
            press(&Backspace, cx),
            Propagation::Continue,
            "empty: launcher's Back"
        );
    }
}
