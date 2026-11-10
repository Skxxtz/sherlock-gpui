use super::*;

/// 1px bar under search bar
pub fn progress_bar(theme: &ThemeData, live: bool) -> AnyElement {
    let track = div()
        .flex_none()
        .h(HAIRLINE)
        .w_full()
        .relative()
        .overflow_hidden();
    if !live {
        return track.into_any_element();
    }
    track
        .bg(theme.border)
        .child(
            div()
                .absolute()
                .top_0()
                .h(HAIRLINE)
                .w(relative(0.25))
                .bg(theme.accent)
                .with_animation(
                    "ask-sweep",
                    Animation::new(Duration::from_millis(1600)).repeat(),
                    |el, t| el.left(relative(t * 1.25 - 0.25)),
                ),
        )
        .into_any_element()
}

pub struct EmptyState<'a> {
    pub model: &'a str,
    /// Recent chats, newest first.
    pub chats: &'a [ChatSummary],
    pub recent_selected: Option<usize>,
    /// "Today: …" and quota, already joined; and whether quota is low.
    pub footer: Option<(String, bool)>,
}

pub fn empty_state(
    theme: &ThemeData,
    view: WeakEntity<AskView>,
    s: EmptyState,
) -> impl IntoElement {
    let small = px(theme.typography.font_size_sm);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let recent = (!s.chats.is_empty()).then(|| {
        let mut list = div()
            .w_full()
            .max_w(EMPTY_MAX_W)
            .flex()
            .flex_col()
            .gap(SPACE_2XS)
            .child(
                div()
                    .px(SPACE_XL)
                    .pb(SPACE_S)
                    .text_size(small)
                    .text_color(theme.tertiary_text)
                    .child("Recent chats"),
            );
        for (i, chat) in s.chats.iter().enumerate() {
            let selected = s.recent_selected == Some(i);
            let hover_bg = theme.bg_selected;
            let (view, delete_view) = (view.clone(), view.clone());
            let group = SharedString::from(format!("ask-chat-{i}"));
            list = list.child(
                div()
                    .id(("ask-recent", i))
                    .group(group.clone())
                    .flex()
                    .items_center()
                    .gap(SPACE_L)
                    .px(SPACE_XL)
                    .py(RECENT_Y)
                    .rounded(px(theme.metrics.radius))
                    .cursor_pointer()
                    .when(selected, |el| el.bg(theme.bg_selected))
                    .hover(move |st| st.bg(hover_bg))
                    .child(
                        div()
                            .flex_none()
                            .text_color(if selected {
                                theme.accent
                            } else {
                                theme.tertiary_text
                            })
                            .child("›"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(if selected {
                                theme.primary_text
                            } else {
                                theme.secondary_text
                            })
                            .child(chat.title.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(small)
                            .text_color(theme.tertiary_text)
                            .child(chat_meta(chat, now)),
                    )
                    .when(selected, |el| el.child(kbd(theme, "⏎")))
                    // Delete: shown on hover and for the focused row.
                    .child(
                        div()
                            .id(("ask-recent-delete", i))
                            .flex_none()
                            .px(SPACE_XS)
                            .rounded(px(theme.metrics.radius))
                            .text_color(theme.tertiary_text)
                            .opacity(if selected { 1. } else { 0. })
                            .group_hover(group, |st| st.opacity(1.))
                            .hover(|st| st.text_color(theme.color_err))
                            .child("×")
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                let _ = delete_view.update(cx, |v, cx| v.delete_chat(i, cx));
                            }),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = view.update(cx, |v, cx| v.open_chat(i, cx));
                    }),
            );
        }
        list
    });

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(EMPTY_GAP)
        .px(EMPTY_X)
        .font_family(theme.typography.font_family.clone())
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(SPACE_S)
                .child(
                    div()
                        .font_family(theme.typography.serif.clone())
                        .text_size(px(theme.typography.font_size * LEAD_SCALE))
                        .text_color(theme.primary_text)
                        .child("Ask anything"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(SPACE_L)
                        .text_size(small)
                        .text_color(theme.tertiary_text)
                        .child(SharedString::from(s.model.to_owned()))
                        .child("·")
                        .child(hint_el(theme, "⏎", "send")),
                ),
        )
        .children(recent)
        .when_some(s.footer, |el, (text, low)| {
            el.child(
                div()
                    .text_size(small)
                    .text_color(if low {
                        theme.primary_text
                    } else {
                        theme.tertiary_text
                    })
                    .child(SharedString::from(text)),
            )
        })
}

pub(super) fn hint_el(theme: &ThemeData, key: &str, what: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(SPACE_S)
        .child(kbd(theme, key))
        .child(SharedString::from(what.to_owned()))
}

pub(super) fn kbd(theme: &ThemeData, key: &str) -> impl IntoElement {
    div()
        .flex_none()
        .px(KBD_X)
        .rounded(KBD_RADIUS)
        .bg(theme.bg_keybind)
        .font_family(theme.typography.monospace.clone())
        .text_size(px(theme.typography.font_size_sm - 1.))
        .text_color(theme.primary_text)
        .child(SharedString::from(key.to_owned()))
}

/// `3 · 2h`: exchanges (when more than one) and how long ago the last one was.
fn chat_meta(chat: &ChatSummary, now: u64) -> SharedString {
    let age = match now.saturating_sub(chat.last) {
        s if s < 60 => "now".to_owned(),
        s if s < 3_600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3_600),
        s => format!("{}d", s / 86_400),
    };
    match chat.exchanges {
        0 | 1 => age.into(),
        n => format!("{n} · {age}").into(),
    }
}
