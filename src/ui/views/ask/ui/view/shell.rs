use gpui::list;

use super::*;
use crate::ui::components::status_bar::StatusBar;

impl SherlockView for AskView {
    type Data = NoData;

    fn chrome(&self) -> Chrome {
        Chrome {
            search_bar: true,
            status_bar: true,
            ..Chrome::NONE
        }
    }

    fn background(&self, _cx: &App) -> Option<Background> {
        let ex = self.conv.current.as_ref()?;
        let (label, busy) = if self.is_running() {
            ("Running command…".into(), BusyStatus::Busy)
        } else if ex.phase.is_live() {
            ("Answering…".into(), BusyStatus::Busy)
        } else if matches!(ex.phase, Phase::Failed(_)) {
            ("Answer failed".into(), BusyStatus::Err)
        } else {
            let q = ex.question.chars().take(28).collect::<String>();
            let more = if ex.question.chars().count() > 28 {
                "…"
            } else {
                ""
            };
            (format!("{q}{more}").into(), BusyStatus::Ok)
        };
        Some(Background { label, busy })
    }

    fn retain(&self) -> bool {
        self.retain
    }

    fn status_bar(&self, _cx: &App) -> Option<StatusBar> {
        self.bar()
    }

    fn placeholder(&self, _cx: &App) -> Option<SharedString> {
        let text = if self.is_running() {
            "Running… Esc stops it"
        } else if self.conv.is_live() {
            "Esc to stop, or type the next question…"
        } else if self.focus.output().is_some() {
            "Ask about the output, or ⌫ to clear it…"
        } else if self.focus.selected_item().is_some() {
            "Enter runs the selection…"
        } else if self.conv.has_unsent_output() {
            "Ask about the output…"
        } else if !self.conv.is_empty() {
            "Ask a follow-up…"
        } else {
            "Ask anything…"
        };
        Some(text.into())
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.input = shell.query.clone();
        if self.shell.is_none() {
            self.shell = Some(shell.shell.downgrade());
        }
        // Typing a new question takes the focus off items and recent chats.
        if !self.input.is_empty() {
            self.focus = Focus::None;
        }
        let theme = &shell.theme;
        self.theme = Some(theme.clone());
        let view = cx.entity().downgrade();

        if self.conv.shown().is_none() {
            let footer = self.empty_footer();
            return render::empty_state(
                theme,
                view,
                EmptyState {
                    model: &self.provider.model,
                    chats: self.chats(),
                    recent_selected: self.focus.recent(),
                    footer,
                },
            )
            .into_any_element();
        }
        // Safety net
        if self.list.item_count() != self.exchange_count() {
            self.reset_list();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(render::progress_bar(theme, self.conv.is_live()))
            .child(
                div().w_full().flex_1().min_h_0().child(
                    list(self.list.clone(), move |ix, _win, cx| {
                        match view.upgrade() {
                            Some(view) => view.update(cx, |view, cx| view.render_exchange(ix, cx)),
                            None => div().into_any_element(),
                        }
                    })
                    .size_full(),
                ),
            )
            .into_any_element()
    }
}
