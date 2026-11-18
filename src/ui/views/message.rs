use std::{cell::Cell, sync::Arc};

use gpui::{
    AnyElement,
    App,
    AppContext,
    Context,
    IntoElement,
    ParentElement,
    Styled,
    Window,
    div,
    px,
};

use crate::{
    app::theme::ActiveTheme,
    config::HomeType,
    features::message::MessageLauncher,
    launcher::{LauncherConfig, variant_type::LauncherType},
    ui::{
        components::status_bar::{StatusBar, StatusItem},
        launcher::navigator::views::Layout,
        model::{
            Model,
            data::ModelData,
            list_data::{ListData, ListId},
            pipeline::FilterPolicy,
        },
        views::{
            Chrome,
            Query,
            QueryResult,
            ShellCtx,
            SherlockView,
            list::list_body,
            query::model_query,
        },
        widgets::message::MessageChild,
    },
    utils::errors::{SherlockMessage, SherlockMessageLevel},
};

mod commands;
mod render;

static MESSAGE_BAR_RIGHT: [StatusItem; 1] = [StatusItem::HINTS];

pub struct MessageView {
    pub launcher_config: Arc<LauncherConfig>,
    pub count: Cell<usize>,
    pub model: Model<ListData<MessageChild>>,
    /// The row expanded last frame: rows change height when the selection moves.
    expanded: Cell<Option<usize>>,
}

impl MessageView {
    pub fn new(data: Vec<SherlockMessage>, cx: &mut Context<Self>) -> Self {
        let config = Arc::new(LauncherConfig {
            name: Some("Errors".into()),
            priority: 1,
            home: HomeType::Home,
            launcher_type: LauncherType::Message(MessageLauncher {}),
            spawn_focus: true,
            ..Default::default()
        });
        let total = data.len();
        let mut messages: Vec<MessageChild> = Vec::with_capacity(total);
        for message in data {
            if let Some(existing) = messages
                .iter_mut()
                .find(|m| m.message.is_repeat_of(&message))
            {
                existing.count += 1;
                message.quiet(); // shown and logged once
                continue;
            }
            let weak = cx.entity().downgrade();
            messages.push(MessageChild::new(message).on_dismiss(move |cx, idx| {
                if let Some(entity) = weak.upgrade() {
                    entity.update(cx, |message_view, cx| {
                        message_view.remove_message(idx, cx);
                    });
                }
            }));
        }

        let count = total;
        let data = ListData::new(messages);
        let all = data.all_ids();
        let entity = cx.new(|_| data);

        Self {
            launcher_config: config,
            count: Cell::new(count),
            model: Model::new(entity, all, FilterPolicy::QueryOnly),
            expanded: Cell::new(None),
        }
    }
    /// This adds a message from the Model. It requires a filter and sort afterwards
    pub fn push_message(&mut self, message: SherlockMessage, cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        self.model.data().update(cx, |this, cx| {
            if let Some(existing) = this
                .items_mut()
                .iter_mut()
                .find(|m| m.message.is_repeat_of(&message))
            {
                existing.count += 1;
                message.quiet(); // shown and logged once
            } else {
                this.push(MessageChild::new(message).on_dismiss(move |cx, id| {
                    if let Some(view) = weak.upgrade() {
                        view.update(cx, |view, cx| view.remove_message(id, cx));
                    }
                }));
            }
            cx.notify();
        });
        self.count.update(|i| i + 1);
    }
    /// This removes a message from the Model. It requires a filter and sort afterwards
    pub fn remove_message(&mut self, idx: ListId, cx: &mut Context<Self>) {
        let Some(removed) = self.model.data().update(cx, |this, _| this.remove(idx)) else {
            return;
        };

        let filtered: Arc<[ListId]> = self
            .model
            .filtered
            .iter()
            .filter(|&&id| id != idx)
            .map(|&id| if id > idx { ListId(id.0 - 1) } else { id })
            .collect();
        let query = self.model.last_query().unwrap_or_default();
        self.model.set_results(filtered, query);

        self.count.update(|i| i.saturating_sub(removed.count));
        cx.notify();
    }

    pub fn clear_messages(&mut self, cx: &mut App) {
        self.model.data.update(cx, |data, _| data.set([]));
        self.model.filtered = Arc::from([]);
        self.count.set(0);
    }
}

impl SherlockView for MessageView {
    type Data = ListData<MessageChild>;

    fn chrome(&self) -> Chrome {
        Chrome {
            sidebar: false,
            mode_label: false,
            inset: false,
            ..Default::default()
        }
    }

    fn commands() -> &'static [crate::ui::commands::Command<Self>] {
        Self::COMMANDS
    }

    fn model(&self) -> Option<&Model<ListData<MessageChild>>> {
        Some(&self.model)
    }

    fn status_bar(&self, cx: &App) -> Option<StatusBar> {
        // left: count per level, like the journal design ("1 err  5 wrn")
        let (mut err, mut wrn, mut inf) = (0, 0, 0);
        for m in self.model.data().read(cx).items() {
            match m.message.level {
                SherlockMessageLevel::Error => err += m.count,
                SherlockMessageLevel::Warning => wrn += m.count,
                SherlockMessageLevel::Info => inf += m.count,
            }
        }
        let counts: Vec<StatusItem> = [(err, "err"), (wrn, "wrn"), (inf, "inf")]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, label)| StatusItem::text(format!("{n} {label}")))
            .collect();

        // right: the view's hinted commands (message.copy, …), keys from the keymap
        Some(
            StatusBar::shell()
                .with_left(counts)
                .with_right(&MESSAGE_BAR_RIGHT[..]),
        )
    }

    fn model_mut(&mut self) -> Option<&mut Model<ListData<MessageChild>>> {
        Some(&mut self.model)
    }

    fn layout(&self) -> Layout {
        Layout::row(self.model.len(), px(32.))
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.model.filtered_indices().is_empty() {
            let theme = cx.global::<ActiveTheme>().0.clone();
            return render::empty_state(&theme).into_any_element();
        }

        // the selected row expands into its trace; the old and new one change height
        let selected = Some(self.model.selected_index());
        if self.expanded.replace(selected) != selected {
            shell.layout.list.remeasure();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(render::column_header(&shell.theme))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(list_body(&self.model, shell, cx)),
            )
            .into_any_element()
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        model_query(&mut self.model, query, cx, |v: &mut Self| &mut v.model)
    }
}
