use gpui::ClipboardItem;

use crate::ui::{
    actions::{Copy, DeleteAll},
    commands::{Command, Key},
    model::data::ModelData,
    views::Propagation,
};

fn done(f: impl FnOnce()) -> Propagation {
    f();
    Propagation::Stop
}

impl super::MessageView {
    pub(super) const COMMANDS: &[Command<Self>] = &[
        Command {
            id: "message.copy",
            label: "Copy Message",
            keys: &[Key::action::<Copy>()],
            when: |v, _| v.model.selected_id().is_some(),
            run: |v, _, cx| {
                let Some(id) = v.model.selected_id() else {
                    return Propagation::Continue;
                };
                let Some(text) = v
                    .model
                    .data()
                    .read(cx)
                    .get(id)
                    .map(|m| m.message.to_report())
                else {
                    return Propagation::Continue;
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                Propagation::Stop
            },
            hint: true,
        },
        Command {
            id: "message.delete",
            label: "Delete Message",
            keys: &[Key::action::<DeleteAll>()],
            when: |v, _| v.model.selected_id().is_some(),
            run: |v, _, cx| {
                if let Some(idx) = v.model.selected_id() {
                    done(|| v.remove_message(idx, cx))
                } else {
                    Propagation::Continue
                }
            },
            hint: true,
        },
    ];
}
