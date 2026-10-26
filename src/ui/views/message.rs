use crate::ui::views::{ShellCtx, SherlockView, list::list_body};
use gpui::{AnyElement, Window};
use std::{cell::Cell, collections::HashMap, sync::Arc};

use gpui::{App, Context, WeakEntity};

use crate::{
    app::data::LauncherData,
    features::message::MessageLauncher,
    launcher::{Launcher, LauncherConfig, variant_type::LauncherType},
    ui::{
        model::{ItemIdx, Model},
        widgets::{ChildKind, RenderableChild, message::MessageChild},
    },
    utils::{config::HomeType, errors::SherlockMessage},
};

pub struct MessageView {
    pub launcher_config: Arc<LauncherConfig>,
    pub count: Cell<usize>,
    pub model: Model,
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
        let messages: Vec<_> = data
            .into_iter()
            .map(|message| {
                let weak = cx.entity().downgrade();
                let inner = MessageChild::new(message).on_dismiss(move |cx, idx| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(cx, |message_view, cx| {
                            message_view.remove_message(idx, cx);
                        });
                    }
                });
                RenderableChild::new(config.clone(), inner)
            })
            .collect();

        let count = messages.len();
        let launchers = HashMap::from([(
            config.id(),
            Launcher {
                config: config.clone(),
                children: messages.into(),
            },
        )]);

        Self {
            launcher_config: config,
            count: Cell::new(count),
            model: Model::standard(launchers, cx),
        }
    }
    /// This adds a message from the Model. It requires a filter and sort afterwards
    pub fn push_message(
        &self,
        message: SherlockMessage,
        weak: WeakEntity<MessageView>,
        cx: &mut App,
    ) {
        let id = self.launcher_config.id();
        self.model.data().update(cx, |this, _| {
            let data = this.make_mut();
            let Some(message_launcher) = data.get_mut(&id) else {
                return;
            };

            // increment existing error
            for item in message_launcher.children.values_mut() {
                if let ChildKind::Message(inner) = &mut item.kind
                    && inner.message == message
                {
                    inner.count += 1;
                    return;
                }
            }

            // no duplicates
            message_launcher.children.push(RenderableChild::new(
                self.launcher_config.clone(),
                MessageChild::new(message).on_dismiss(move |cx, idx| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(cx, |message_view, cx| {
                            message_view.remove_message(idx, cx);
                        });
                    }
                }),
            ));
        });
        self.count.update(|i| i + 1);
    }
    /// This removes a message from the Model. It requires a filter and sort afterwards
    pub fn remove_message(&mut self, idx: ItemIdx, cx: &mut App) {
        let data = self.model.data();

        let removed = data.update(cx, |this, _| {
            if this
                .get(&idx.0)
                .is_some_and(|l| l.children.get(idx.1).is_some())
            {
                this.make_mut()
                    .get_mut(&idx.0)
                    .and_then(|l| l.children.remove(idx.1))
            } else {
                None
            }
        });

        if let Some(ChildKind::Message(inner)) = removed.as_ref().map(|r| &r.kind) {
            self.model.filtered = self
                .model
                .filtered
                .iter()
                .copied()
                .filter(|&x| x != idx)
                .collect();
            self.count.update(|i| i.saturating_sub(inner.count));
        }
    }
    pub fn clear_messages(&mut self, cx: &mut App) {
        self.model
            .data
            .update(cx, |this, _| *this = LauncherData::default());
        self.model.filtered = Arc::from([]);
        self.count.set(0);
    }
}

impl SherlockView for MessageView {
    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        list_body(&self.model, shell, cx)
    }
}
