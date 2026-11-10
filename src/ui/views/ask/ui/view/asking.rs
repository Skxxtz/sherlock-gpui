use super::*;

impl AskView {
    /// Ask a new question.
    pub(in super::super) fn ask(&mut self, question: SharedString, cx: &mut Context<Self>) {
        self.stop(cx);
        self.stop_run(cx);
        self.focus = Focus::None;
        self.text_sel = None;

        let messages = self.conv.begin(question, &self.system_prompt);
        self.show_new_exchange();

        match &self.key {
            KeyState::Ready => self.start_stream(messages, cx),
            KeyState::Pending => self.queued = Some(messages),
            KeyState::Failed(e) => {
                let e = e.clone();
                if let Some(ex) = self.conv.current.as_mut() {
                    ex.fail(e);
                }
            }
        }
        self.repaint(cx);
    }

    pub(super) fn start_stream(&mut self, messages: Vec<Message>, cx: &mut Context<Self>) {
        match AnswerStream::start(&self.provider, messages) {
            Some(stream) => {
                self.stream = Some(stream);
                self.pump = Some(poll_every(FRAME, cx, |v, cx| v.drain(cx) != Drain::Ended));
            }
            None => {
                if let Some(ex) = self.conv.current.as_mut() {
                    ex.fail("Sherlock's async runtime isn't available.");
                }
            }
        }
    }

    /// The key command finished.
    pub(super) fn key_read(&mut self, key: Result<String, String>, cx: &mut Context<Self>) {
        match key {
            Ok(key) => {
                self.provider.api_key = Some(key);
                self.key = KeyState::Ready;
                if let Some(messages) = self.queued.take() {
                    self.start_stream(messages, cx);
                }
            }
            Err(e) => {
                let e = SharedString::from(e);
                if self.queued.take().is_some()
                    && let Some(ex) = self.conv.current.as_mut()
                {
                    ex.fail(e.clone());
                }
                self.key = KeyState::Failed(e);
            }
        }
        self.repaint(cx);
    }

    pub(super) fn drain(&mut self, cx: &mut Context<Self>) -> Drain {
        let (Some(stream), Some(ex)) = (self.stream.as_mut(), self.conv.current.as_mut()) else {
            return Drain::Ended;
        };
        let state = stream.drain_into(ex);
        if state == Drain::Ended {
            self.stream = None;
            self.save(cx);
            // A lone action is the answer itself: preselect it, so Enter runs it.
            if let [Item::Action(_)] = &*self.items() {
                self.focus = Focus::Item(0);
            }
        }
        if state != Drain::Idle {
            // Only the streaming exchange changed.
            self.remeasure_current();
            self.repaint(cx);
        }
        state
    }

    /// Stops a live answer and keeps what arrived so far.
    pub(in super::super) fn stop(&mut self, cx: &mut Context<Self>) {
        self.pump = None;
        let waiting = self.queued.take().is_some();
        if (self.stream.take().is_some() || waiting)
            && let Some(ex) = self.conv.current.as_mut()
        {
            ex.stop();
            self.save(cx);
        }
    }

    /// Moves back to the ask launcher home screen
    pub(in super::super) fn new_conversation(&mut self, cx: &mut Context<Self>) {
        self.stop(cx);
        self.stop_run(cx);
        self.conv.reset();
        self.reset_list();
        self.focus = Focus::None;
        self.text_sel = None;
        self.repaint(cx);
    }

    /// Display and navigate older chats.
    pub(in super::super) fn browse(&mut self, earlier: bool, cx: &mut Context<Self>) {
        let moved = if earlier {
            self.conv.view_earlier()
        } else {
            self.conv.view_later()
        };
        if moved {
            self.text_sel = None;
            self.focus = if earlier {
                Focus::None
            } else {
                self.stops().last().unwrap_or_default()
            };
            self.scroll_to_exchange(self.list_index(self.conv.viewing));
            self.repaint(cx);
        }
    }

    /// Appends the current exchange to the history, off the UI thread.
    pub(super) fn save(&mut self, cx: &mut Context<Self>) {
        let (Some(history), Some(ex)) = (self.history.clone(), self.conv.current.as_ref()) else {
            return;
        };
        let Some(entry) = Entry::from_exchange(ex, &self.provider.model, self.conv.id) else {
            return;
        };
        if let Some(today) = &mut self.today {
            today.add(entry.usage);
        }
        let entry = entry.into_owned();
        let question = ex.question.clone();
        self.note_saved(self.conv.id, &question);
        cx.background_executor()
            .spawn(async move {
                let _ = history.append(&entry);
            })
            .detach();
    }
}
