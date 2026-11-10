use gpui::{ListOffset, Pixels};

use super::*;
use crate::ui::views::ask::ui::tokens::{EXCHANGE_GAP, OPACITY_DIM, PAGE_BOTTOM, PAGE_TOP, PAGE_X};

/// Offsets the element and selection ids of exchange `i`, so ids stay unique across the list.
const IDS_PER_EXCHANGE: usize = 1 << 16;

impl AskView {
    /// Number of exchanges in the list.
    pub(super) fn exchange_count(&self) -> usize {
        self.conv.turns().len() + usize::from(self.conv.current.is_some())
    }

    /// The exchange at list position `ix` (newest first), and its position in the chat (oldest
    /// first).
    fn exchange(&self, ix: usize) -> Option<(&Exchange, usize)> {
        let turns = self.conv.turns().len();
        match (ix, &self.conv.current) {
            (0, Some(current)) => Some((current, turns)),
            _ => {
                // 0 if none exists.
                let k = ix + 1 - usize::from(self.conv.current.is_some());
                let i = turns.checked_sub(k)?;
                Some((self.conv.turns().get(i)?, i))
            }
        }
    }

    /// List position of `conv.turns[i]`, or of the current exchange for `None`.
    pub(super) fn list_index(&self, turn: Option<usize>) -> usize {
        match turn {
            None => 0,
            Some(i) => self.conv.turns().len() - i - 1 + usize::from(self.conv.current.is_some()),
        }
    }

    /// Focused item follows the scroll position
    pub(super) fn track_focus(list: &ListState, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        list.set_scroll_handler(move |_, _, cx| {
            let view = view.clone();
            cx.defer(move |cx| {
                view.update(cx, |view, cx| {
                    let focused = view.focused_exchange();
                    if focused != view.focused {
                        view.focused = focused;
                        cx.notify();
                    }
                })
                .ok();
            });
        });
    }

    /// Rebuilds the list.
    pub(super) fn reset_list(&mut self) {
        self.list.reset(self.exchange_count());
        self.focused = 0;
    }

    /// On new message, focus top
    pub(super) fn show_new_exchange(&mut self) {
        self.reset_list();
        self.scroll_to_exchange(0);
    }

    /// The newest exchange changed height.
    pub(super) fn remeasure_current(&self) {
        if self.conv.current.is_some() {
            self.list.remeasure_items(0..1);
        }
    }

    /// Exchange `at` changed height.
    pub(super) fn remeasure(&self, at: Option<usize>) {
        if at.is_some() || self.conv.current.is_some() {
            let ix = self.list_index(at);
            self.list.remeasure_items(ix..ix + 1);
        }
    }

    /// Scroll wheel support.
    pub(in super::super) fn scroll_chat(&mut self, dy: Pixels, cx: &mut Context<Self>) {
        self.list.scroll_by(-dy);
        self.focused = self.focused_exchange();
        self.repaint(cx);
    }

    /// Scrolls so exchange `ix` starts at the top.
    pub(super) fn scroll_to_exchange(&mut self, ix: usize) {
        self.focused = ix;
        self.list.scroll_to(ListOffset {
            item_ix: ix,
            offset_in_item: px(0.),
        });
    }

    /// List position of the exchange being read.
    /// **Safety**: never call from the list's own callbacks.
    fn focused_exchange(&self) -> usize {
        let top = self.list.logical_scroll_top();
        let past_half = self
            .list
            .bounds_for_item(top.item_ix)
            .is_some_and(|b| top.offset_in_item > b.size.height / 2.);
        let ix = top.item_ix + usize::from(past_half);
        ix.min(self.exchange_count().saturating_sub(1))
    }

    /// One exchange as a list item.
    pub(super) fn render_exchange(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let (Some((ex, pos)), Some(theme)) = (self.exchange(ix), self.theme.as_ref()) else {
            return div().into_any_element();
        };
        let focused = self.conv.shown().is_some() && ix == self.list_index(self.conv.viewing);
        let idle_scroll = ScrollHandle::new();
        let base = pos * IDS_PER_EXCHANGE;
        let mut ui = Ui {
            theme,
            view: cx.entity().downgrade(),
            apps: &self.targets.apps,
            ready: focused && !ex.phase.is_live(),
            focus: if focused { self.focus } else { Focus::None },
            text_sel: self.text_sel.as_ref(),
            runs: &ex.runs,
            output_scroll: if focused {
                &self.output_scroll
            } else {
                &idle_scroll
            },
            text_index: base,
            code_index: base,
            runnable_index: 0,
        };
        let blocks = ex.blocks();
        let answer = render::answer(ex, &blocks, &mut ui);
        let count = self.exchange_count();
        let last = ix + 1 == count;
        let dimmed = count > 1 && ix != self.focused;

        div()
            .w_full()
            .px(PAGE_X)
            .when(ix == 0, |el| el.pt(PAGE_TOP))
            .pb(if last { PAGE_BOTTOM } else { EXCHANGE_GAP })
            .child(
                div()
                    .w_full()
                    .when(ix > 0, |el| {
                        el.border_t_1().border_color(theme.border).pt(EXCHANGE_GAP)
                    })
                    .child(
                        div()
                            .w_full()
                            .when(dimmed, |el| el.opacity(OPACITY_DIM))
                            .child(answer),
                    ),
            )
            .into_any_element()
    }
}
