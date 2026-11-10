use super::*;

impl AskView {
    pub(in super::super) fn begin_text_selection(
        &mut self,
        block: usize,
        text: SharedString,
        at: usize,
        clicks: usize,
        cx: &mut Context<Self>,
    ) {
        self.text_sel = Some(TextSelection::start(block, text, at, clicks));
        self.repaint(cx);
    }

    pub(in super::super) fn extend_text_selection(
        &mut self,
        block: usize,
        at: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(sel) = self
            .text_sel
            .as_mut()
            .filter(|s| s.dragging && s.block == block)
        {
            let before = sel.range();
            sel.extend_to(at);
            if sel.range() != before {
                self.repaint(cx);
            }
        }
    }

    pub(in super::super) fn end_text_selection(&mut self, cx: &mut Context<Self>) {
        let Some(sel) = self.text_sel.as_mut().filter(|s| s.dragging) else {
            return;
        };
        sel.dragging = false;
        if sel.is_empty() {
            self.text_sel = None;
        } else {
            cx.write_to_primary(ClipboardItem::new_string(sel.selected_text().to_owned()));
        }
        self.repaint(cx);
    }

    pub(in super::super) fn copy_to_clipboard(
        &mut self,
        text: String,
        toast: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.show_toast(toast, cx);
    }

    pub(in super::super) fn show_toast(&mut self, text: &'static str, cx: &mut Context<Self>) {
        self.toast = Some(text.into());
        self.toast_task = Some(cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                cx.background_executor().timer(TOAST).await;
                let _ = this.update(&mut cx, |view, cx| {
                    view.toast = None;
                    view.repaint(cx);
                });
            }
        }));
        self.repaint(cx);
    }
}
