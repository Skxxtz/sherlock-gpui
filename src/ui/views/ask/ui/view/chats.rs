use super::*;

impl AskView {
    /// The chats offered on the empty view, newest first.
    pub(in super::super) fn chats(&self) -> &[ChatSummary] {
        &self.recent[..self.recent.len().min(self.num_recent_chats)]
    }

    /// Reopen a stored chat
    pub(in super::super) fn open_chat(&mut self, index: usize, cx: &mut Context<Self>) {
        let (Some(chat), Some(history)) = (self.chats().get(index), self.history.clone()) else {
            return;
        };
        let conv = chat.conv;
        self.stop(cx);
        self.stop_run(cx);
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let entries = cx
                    .background_executor()
                    .spawn(
                        async move { history.conversation(conv, MAX_HISTORY).unwrap_or_default() },
                    )
                    .await;
                let _ = this.update(&mut cx, |view, cx| {
                    let exchanges: Vec<Exchange> = entries
                        .into_iter()
                        .map(|e| Exchange::loaded(e.question.into_owned(), e.answer, e.usage))
                        .collect();
                    if exchanges.is_empty() {
                        view.show_toast("Chat not found", cx);
                        return;
                    }
                    view.conv = Conversation::restore(conv, exchanges, view.conv.max_turns());
                    view.focus = Focus::None;
                    view.text_sel = None;
                    // The newest answer on top; earlier ones below it.
                    view.show_new_exchange();
                    view.repaint(cx);
                });
            }
        })
        .detach();
    }

    /// Deletes a chat from the list and the cache
    pub(in super::super) fn delete_chat(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.chats().len() {
            return;
        }
        let chat = self.recent.remove(index);
        if let Some(history) = self.history.clone() {
            cx.background_executor()
                .spawn(async move {
                    let _ = history.delete_conversation(chat.conv);
                })
                .detach();
        }
        self.focus = match self.chats().len() {
            0 => Focus::None,
            n => Focus::Recent(index.min(n - 1)),
        };
        self.show_toast("Chat deleted", cx);
    }

    /// A saved exchange moves its chat to the top of the list.
    pub(super) fn note_saved(&mut self, conv: u64, question: &SharedString) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let chat = match self.recent.iter().position(|c| c.conv == conv) {
            Some(i) => {
                let mut chat = self.recent.remove(i);
                chat.last = now;
                chat.exchanges += 1;
                chat
            }
            None => ChatSummary {
                conv,
                title: question.clone(),
                last: now,
                exchanges: 1,
            },
        };
        self.recent.insert(0, chat);
        self.recent.truncate(MAX_LOADED_CHATS);
    }
}
