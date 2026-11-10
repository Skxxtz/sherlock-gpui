use gpui::{
    Animation,
    AnimationExt,
    AnyElement,
    Div,
    ElementId,
    IntoElement,
    ParentElement,
    SharedString,
    Styled,
    div,
    pulsating_between,
    px,
    relative,
};

use super::StatusKind;
use crate::{app::theme::ThemeData, ui::views::BusyStatus};

/// Height of the bar.
pub const HEIGHT: f32 = 30.;
/// Gap between items.
pub const GAP: f32 = 10.;

/// A key cap.
pub fn key_cap(key: impl Into<SharedString>, theme: &ThemeData) -> Div {
    div()
        .flex_none()
        .p(px(5.))
        .bg(theme.bg_keybind)
        .rounded_sm()
        .text_size(px(11.))
        .line_height(relative(1.0))
        .child(key.into())
}

/// Key caps followed by their label.
pub fn keys<K: Into<SharedString>>(
    keys: impl IntoIterator<Item = K>,
    label: SharedString,
    theme: &ThemeData,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .children(keys.into_iter().map(|k| key_cap(k, theme)))
        .child(div().ml(px(2.)).child(label))
}

/// A dot in the status's color, pulsing while busy.
pub fn status_dot(status: BusyStatus, id: impl Into<ElementId>, theme: &ThemeData) -> AnyElement {
    let dot = div().flex_none().size(px(6.)).rounded_full();
    match status {
        BusyStatus::Ok => dot.bg(theme.color_succ).into_any_element(),
        BusyStatus::Err => dot.bg(theme.color_err).into_any_element(),
        BusyStatus::Busy => dot
            .bg(theme.accent)
            .with_animation(
                id,
                Animation::new(std::time::Duration::from_millis(1200))
                    .repeat()
                    .with_easing(pulsating_between(0.3, 1.0)),
                |el, t| el.opacity(t),
            )
            .into_any_element(),
    }
}

/// A view item; `None` for shell items, which the shell draws. `id` keeps animations apart.
pub fn view_item(kind: &StatusKind, id: usize, theme: &ThemeData) -> Option<Div> {
    Some(match kind {
        StatusKind::Text(text) => div().child(text.clone()),
        StatusKind::Keys { keys: k, label } => keys(k.iter().copied(), label.clone(), theme),
        StatusKind::Status { label, status } => div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(status_dot(
                *status,
                ElementId::NamedInteger("status-bar-dot".into(), id as u64),
                theme,
            ))
            .child(label.clone()),
        StatusKind::Brand
        | StatusKind::Messages
        | StatusKind::Minimized
        | StatusKind::ContextActions => return None,
    })
}
