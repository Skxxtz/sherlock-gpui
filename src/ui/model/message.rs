use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui::{App, Context, WeakEntity};

use crate::{
    launcher::{
        Launcher, LauncherConfig, message_launcher::MessageLauncher, variant_type::LauncherType,
    },
    ui::{
        model::Model,
        widgets::{RenderableChild, message::MessageChild},
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
            icon: None,
            alias: None,
            on_return: None,
            exit: false,
            priority: 1,
            limit: None,
            home: HomeType::Home,
            launcher_type: LauncherType::Message(MessageLauncher {}),
            shortcut: false,
            spawn_focus: true,
            actions: None,
            add_actions: None,
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
                RenderableChild::Message {
                    launcher: Arc::clone(&config),
                    inner,
                }
            })
            .collect();

        let count = messages.len();
        let launcher_vec = vec![Launcher {
            config: config.clone(),
            children: messages,
        }];

        Self {
            launcher_config: config,
            count: Cell::new(count),
            model: Model::standard(launcher_vec, cx),
        }
    }
    /// This adds a message from the Model. It requires a filter and sort afterwards
    pub fn push_message(
        &self,
        message: SherlockMessage,
        weak: WeakEntity<MessageView>,
        cx: &mut App,
    ) {
        self.model.data().update(cx, |this, _| {
            let data = Rc::make_mut(this);
            let message_launcher = &mut data[0];

            // increment existing error
            for item in message_launcher.children.iter_mut() {
                if let RenderableChild::Message { inner, .. } = item
                    && inner.message == message
                {
                    inner.count += 1;
                    return;
                }
            }

            // no duplicates
            message_launcher.children.push(RenderableChild::Message {
                launcher: self.launcher_config.clone(),
                inner: MessageChild::new(message).on_dismiss(move |cx, idx| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(cx, |message_view, cx| {
                            message_view.remove_message(idx, cx);
                        });
                    }
                }),
            });
        });
        self.count.update(|i| i + 1);
    }
    /// This removes a message from the Model. It requires a filter and sort afterwards
    pub fn remove_message(&mut self, idx: (usize, usize), cx: &mut App) {
        let Model::Standard {
            data,
            filtered_indices,
            ..
        } = &mut self.model
        else {
            return;
        };

        let removed = data.update(cx, |this, _| {
            if this.get(idx.0).is_some_and(|l| idx.1 < l.children.len()) {
                let data = Rc::make_mut(this);
                data.get_mut(idx.0).map(|l| l.children.remove(idx.1))
            } else {
                None
            }
        });

        if let Some(RenderableChild::Message { inner, .. }) = removed {
            let mut vec = filtered_indices.to_vec();
            if let Some(pos) = vec.iter().position(|&x| x == idx) {
                vec.remove(pos);
            }

            for val in vec.iter_mut() {
                if val.1 > idx.1 {
                    val.1 -= 1;
                }
            }

            *filtered_indices = Arc::from(vec);
            self.count.update(|i| i.saturating_sub(inner.count));
        }
    }
    pub fn clear_messages(&mut self, cx: &mut App) {
        let Model::Standard {
            data,
            filtered_indices,
            ..
        } = &mut self.model
        else {
            return;
        };

        data.update(cx, |this, _| *this = Rc::new(Vec::new()));
        *filtered_indices = Arc::from([]);

        self.count.set(0);
    }
}
