use super::*;
use crate::ui::views::ask::model::items::Script;

impl AskView {
    pub(in super::super) fn select_item(&mut self, item: usize, cx: &mut Context<Self>) {
        self.set_focus(Focus::Item(item), cx);
    }

    pub(in super::super) fn focus_output(&mut self, item: usize, cx: &mut Context<Self>) {
        self.set_focus(Focus::Output(item), cx);
    }

    /// Runs an item.
    pub(in super::super) fn run_item(
        &mut self,
        item: usize,
        terminal: bool,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(it) = self.items().get(item).cloned() else {
            return;
        };
        self.set_focus(Focus::None, cx);
        match it {
            Item::Script(script) if !terminal && !script.terminal => {
                self.start_run(item, script, cx);
            }
            Item::Script(script) => {
                run_in_terminal(&script.text, script.shell);
                self.show_toast("Opened in terminal", cx);
            }
            Item::Action(action) => {
                let (Some(mode), Some(shell)) =
                    (self.targets.exec_mode(action), self.shell.clone())
                else {
                    return;
                };
                // The shell may be mid-dispatch (it called us); run once it's done.
                cx.defer_in(win, move |_, win, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        let req = ExecRequest {
                            mode,
                            exit: true,
                            ..Default::default()
                        };
                        shell.run(req, win, cx);
                    });
                });
            }
        }
    }

    /// Focusable elements.
    pub(in super::super) fn stops(&self) -> impl Iterator<Item = Focus> + Clone + '_ {
        let runs: &[CommandRun] = self.conv.shown().map_or(&[], |ex| &ex.runs);
        focus::stops(
            self.items().len(),
            move |i| runs.iter().any(|r| r.item == i),
            self.suggestions().len(),
        )
    }

    /// Move the focus. Overflow focuses none
    pub(in super::super) fn cycle(&mut self, forward: bool, cx: &mut Context<Self>) {
        let next = focus::cycle(self.focus, self.stops(), forward);
        self.set_focus(next, cx);
    }

    pub(in super::super) fn set_focus(&mut self, focus: Focus, cx: &mut Context<Self>) {
        if self.focus != focus {
            self.focus = focus;
            self.repaint(cx);
        }
    }

    /// The run whose output panel has the focus.
    pub(in super::super) fn focused_output(&self) -> Option<&CommandRun> {
        let item = self.focus.output()?;
        self.conv.shown()?.runs.iter().find(|r| r.item == item)
    }

    /// Handles error output of system command. Asks the model to fix it and sends the error along.
    pub(in super::super) fn fix_run(&mut self, item: usize, cx: &mut Context<Self>) {
        let Some(run) = self.conv.shown().and_then(|ex| {
            ex.runs
                .iter()
                .find(|r| r.item == item && r.failed() && !r.sent)
        }) else {
            return;
        };
        if run.wants_terminal() {
            run_in_terminal(&run.script, run.shell);
            self.show_toast("Opened in terminal", cx);
            return;
        }
        let question = run.fix_question();
        self.ask(question.into(), cx);
        self.input = SharedString::default();
        self.clear_input(cx);
    }
}

// Sherlock's internal command execution
impl AskView {
    pub(super) fn start_run(&mut self, item: usize, script: Script, cx: &mut Context<Self>) {
        self.stop_run(cx);
        let mut run = CommandRun::new(item, script.text.clone()).with_shell(script.shell);
        match RunHandle::spawn(&script.text, script.shell) {
            Ok(handle) => {
                self.run = Some(handle);
                self.run_pump = Some(poll_every(FRAME, cx, |v, cx| {
                    v.drain_run(cx) != RunDrain::Ended
                }));
            }
            Err(e) => run.state = RunState::Failed(e),
        }

        self.run_at = self.conv.viewing;
        let Some(ex) = self.conv.shown_mut() else {
            return;
        };
        ex.runs.retain(|r| r.item != item);
        ex.runs.push(run);

        // Set focus to the output
        self.focus = Focus::Output(item);
        self.output_scroll.set_offset(point(px(0.), px(0.)));
        self.remeasure(self.run_at);
        self.repaint(cx);
    }

    pub(super) fn drain_run(&mut self, cx: &mut Context<Self>) -> RunDrain {
        let (Some(handle), Some(run)) = (
            self.run.as_mut(),
            self.conv
                .get_mut(self.run_at)
                .and_then(|ex| ex.runs.last_mut()),
        ) else {
            return RunDrain::Ended;
        };
        let state = handle.drain_into(run);
        if state == RunDrain::Ended {
            self.run = None;
        }
        // Repaint even without new output: the elapsed time keeps ticking.
        self.output_scroll.scroll_to_bottom();
        self.remeasure(self.run_at);
        self.repaint(cx);
        state
    }

    /// Kills a running command. Its output so far stays.
    pub(in super::super) fn stop_run(&mut self, cx: &mut Context<Self>) {
        self.run_pump = None;
        if let Some(handle) = self.run.take() {
            handle.kill();
            if let Some(run) = self
                .conv
                .get_mut(self.run_at)
                .and_then(|ex| ex.runs.last_mut())
            {
                run.state = RunState::Killed;
                run.elapsed = run.started.elapsed();
            }
            self.repaint(cx);
        }
    }

    /// Removes a run's output (and stops it), so it isn't sent with the next question.
    pub(in super::super) fn clear_run(&mut self, item: usize, cx: &mut Context<Self>) {
        let running = self.is_running()
            && self.run_at == self.conv.viewing
            && self
                .conv
                .shown()
                .and_then(|ex| ex.runs.last())
                .is_some_and(|r| r.item == item && r.is_running());
        if running {
            self.stop_run(cx);
        }
        if let Some(ex) = self.conv.shown_mut() {
            ex.runs.retain(|r| r.item != item);
        }
        if self.focus == Focus::Output(item) {
            self.focus = Focus::Item(item);
        }
        self.remeasure(self.conv.viewing);
        self.repaint(cx);
    }
}
