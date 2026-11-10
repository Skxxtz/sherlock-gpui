#[cfg(feature = "ai")]
pub(super) fn register(cx: &mut gpui::App) {
    cx.text_system()
        .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
            "../assets/fonts/Newsreader-Regular.ttf"
        ))])
        .ok();
}
