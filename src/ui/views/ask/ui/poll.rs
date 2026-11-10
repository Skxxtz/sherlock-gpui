use std::time::Duration;

use gpui::{AsyncApp, Context, Task, WeakEntity};

/// Calls `tick` every `every` until it returns `false` or the view is gone. Dropping the task
/// stops it.
pub(super) fn poll_every<V: 'static>(
    every: Duration,
    cx: &mut Context<V>,
    mut tick: impl FnMut(&mut V, &mut Context<V>) -> bool + 'static,
) -> Task<()> {
    cx.spawn(move |this: WeakEntity<V>, cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            loop {
                cx.background_executor().timer(every).await;
                if !matches!(this.update(&mut cx, |view, cx| tick(view, cx)), Ok(true)) {
                    return;
                }
            }
        }
    })
}
