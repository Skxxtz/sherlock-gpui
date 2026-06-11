use gpui::{App, AppContext, AsyncApp, Render, div};

struct WarmupWindow;
impl Render for WarmupWindow {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
    }
}

/// Warmup trick: Force GPUI to render an empty window wich is destroyed immediately after. This
/// fixed a common issue where the first Sherlock call takes multiple seconds.
pub fn warmup(cx: &mut App) {
    cx.spawn(|cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            let window = cx
                .open_window(gpui::WindowOptions::default(), |_, cx| {
                    cx.new(|_| WarmupWindow)
                })
                .unwrap();
            let _ = window.update(&mut cx, |_, win, _| {
                win.remove_window();
            });
        }
    })
    .detach();
}
