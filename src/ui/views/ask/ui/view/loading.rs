use std::{path::Path, sync::Arc};

use super::*;

impl AskView {
    /// Runs the key command to get the api key.
    pub(super) fn load_key(cmd: String, cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let key = cx
                    .background_executor()
                    .spawn(async move { read_key(&cmd) })
                    .await;
                let _ = this.update(&mut cx, |view, cx| view.key_read(key, cx));
            }
        })
        .detach();
    }

    /// Trims the history file and reads the quota day's totals and the recent chats.
    pub(super) fn load_history(history: History, since: u64, cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let (stats, recent) = cx
                    .background_executor()
                    .spawn(async move {
                        let _ = history.compact();
                        let stats = history.stats_since(since, MAX_HISTORY).unwrap_or_default();
                        let chats = history
                            .recent_chats(MAX_LOADED_CHATS, MAX_HISTORY)
                            .unwrap_or_default();
                        (stats, chats)
                    })
                    .await;
                let _ = this.update(&mut cx, |view, cx| {
                    view.today = Some(stats);
                    view.recent = recent;
                    view.repaint(cx);
                });
            }
        })
        .detach();
    }

    /// Reads what kind of app each one is (browser, editor, …) from its desktop file, so
    /// `open browser` works without sending the app list to the model.
    pub(super) fn load_app_tags(desktop_files: Vec<Arc<Path>>, cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let tags = cx
                    .background_executor()
                    .spawn(async move { Targets::read_tags(&desktop_files) })
                    .await;
                let _ = this.update(&mut cx, |view, _| {
                    view.targets = Rc::new(view.targets.with_tags(tags));
                });
            }
        })
        .detach();
    }
}
