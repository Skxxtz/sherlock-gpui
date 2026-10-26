use gpui::{App, AppContext, Entity, px};

use super::{Layout, NavView, ViewHandle, ViewKind};
use crate::{ui::views::message::MessageView, utils::errors::SherlockMessage};

impl NavView {
    /// Creates the message view, which shows errors and other messages.
    /// Also returns the typed entity, so the navigator can push and clear
    /// messages directly.
    pub(crate) fn messages(
        messages: Vec<SherlockMessage>,
        cx: &mut App,
    ) -> (Self, Entity<MessageView>) {
        let len = messages.len();
        let entity = cx.new(|cx| MessageView::new(messages, cx));
        let view = Self::new(
            ViewHandle::Messages(entity.clone()),
            Layout::row(len, px(100.)),
            ViewKind::Messages,
        );
        (view, entity)
    }
}
