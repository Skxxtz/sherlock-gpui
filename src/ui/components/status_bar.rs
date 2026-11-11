//! The status bar at the bottom of the launcher, as data.
//!
//! A view describes its bar with `SherlockView::status_bar`; the shell draws it every frame, so
//! the bar follows the view's state. `None` gives `StatusBar::default()`.
//!
//! Items are either shell items, filled in by the shell when drawn (`Brand`, `Messages`,
//! `Minimized`, `ContextActions`, `Hints`; hidden when there's nothing to show), or the view's
//! own (`text`, `keys`, `status`), optionally clickable.
//!
//! `Hints` are the view's available commands marked `hint` (see `ui::commands`), with the keys
//! the view's binds or the keymap give them; clicking one runs the command.
//!
//! Key hints name *what* they press (`KeySpec`), not the keys: the shell looks the keys up in
//! the keymap when drawing, so hints follow the user's config. A hint whose key isn't bound is
//! hidden.
//!
//! Cheap per frame: the default bar and fixed bars are statics (`Cow::Borrowed`), click
//! handlers are plain `fn` pointers, labels are `SharedString`s (static or ref-counted).
//!
//! ```ignore
//! static ASK_BAR: [StatusItem; 2] = [
//!     StatusItem::keys(KeySpec::action::<Execute>(), "run").on_click(dispatch::<Execute>),
//!     StatusItem::HINTS,
//! ];
//!
//! fn status_bar(&self, _cx: &App) -> Option<StatusBar> {
//!     Some(StatusBar::shell().with_right(&ASK_BAR))
//! }
//! ```

use std::{borrow::Cow, time::Duration};

use gpui::{
    Action,
    Animation,
    AnimationExt,
    AnyElement,
    App,
    Div,
    ElementId,
    IntoElement,
    Modifiers,
    ParentElement,
    SharedString,
    Styled,
    Window,
    div,
    pulsating_between,
    px,
    relative,
};

use crate::{app::theme::ThemeData, ui::views::BusyStatus};

/// Height of the bar.
pub const HEIGHT: f32 = 30.;
/// Gap between items.
pub const GAP: f32 = 10.;

/// Runs on click, e.g. `|win, cx| win.dispatch_action(Box::new(Quit), cx)`.
pub type OnClick = fn(&mut Window, &mut App);

/// What the bar shows: items from the left edge, and items flush right.
#[derive(Clone)]
pub struct StatusBar {
    pub left: Cow<'static, [StatusItem]>,
    pub right: Cow<'static, [StatusItem]>,
}

static SHELL_LEFT: [StatusItem; 3] = [
    StatusItem::BRAND,
    StatusItem::MESSAGES,
    StatusItem::MINIMIZED,
];
static DEFAULT_RIGHT: [StatusItem; 1] = [StatusItem::CONTEXT_ACTIONS];

/// The launcher's bar: brand, messages and a minimized view on the left; the context menu
/// hint on the right. Allocates nothing.
impl Default for StatusBar {
    fn default() -> Self {
        Self {
            left: Cow::Borrowed(&SHELL_LEFT),
            right: Cow::Borrowed(&DEFAULT_RIGHT),
        }
    }
}

impl StatusBar {
    /// Nothing at all.
    pub const fn empty() -> Self {
        Self {
            left: Cow::Borrowed(&[]),
            right: Cow::Borrowed(&[]),
        }
    }

    /// The shell's items on the left (brand, messages, minimized view); the right side is the
    /// view's.
    pub fn shell() -> Self {
        Self {
            left: Cow::Borrowed(&SHELL_LEFT),
            right: Cow::Borrowed(&[]),
        }
    }

    /// Replaces the right side; a static slice costs nothing, a `Vec` is moved in.
    pub fn with_right(mut self, items: impl Into<Cow<'static, [StatusItem]>>) -> Self {
        self.right = items.into();
        self
    }

    /// Replaces the left side.
    pub fn with_left(mut self, items: impl Into<Cow<'static, [StatusItem]>>) -> Self {
        self.left = items.into();
        self
    }

    /// Adds an item on the right (copies a static side once).
    pub fn right(mut self, item: StatusItem) -> Self {
        self.right.to_mut().push(item);
        self
    }

    /// Adds an item on the left (copies a static side once).
    pub fn left(mut self, item: StatusItem) -> Self {
        self.left.to_mut().push(item);
        self
    }
}

/// One thing in the bar, optionally clickable.
#[derive(Clone, Debug)]
pub struct StatusItem {
    pub kind: StatusKind,
    pub on_click: Option<OnClick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusKind {
    // Filled in by the shell.
    /// "Sherlock"; click shows the credits.
    Brand,
    /// Count of errors and warnings; click opens them. Hidden without messages.
    Messages,
    /// The minimized view's status; click restores it. Hidden without one.
    Minimized,
    /// "Additional Actions" and the context menu key, when the selection has actions.
    ContextActions,
    /// The view's available commands marked `hint`; click runs one. Hidden without any.
    Hints,

    // The view's own.
    /// Plain text.
    Text(SharedString),
    /// Key caps and what they do: `Esc stop`.
    Keys { keys: KeySpec, label: SharedString },
    /// A colored dot (pulsing while busy) and a label.
    Status {
        label: SharedString,
        status: BusyStatus,
    },
}

/// Which keys a `Keys` hint shows, resolved when drawn.
#[derive(Clone, Copy)]
pub enum KeySpec {
    /// Always these caps.
    Fixed(&'static [&'static str]),
    /// Whatever the keymap binds to this action (a `fn`, so hints can live in statics).
    Action(fn() -> Box<dyn Action>),
}

impl KeySpec {
    /// The keys bound to `A`.
    pub const fn action<A: Action + Default>() -> Self {
        Self::Action(boxed::<A>)
    }
}

fn boxed<A: Action + Default>() -> Box<dyn Action> {
    Box::new(A::default())
}

impl PartialEq for KeySpec {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fixed(a), Self::Fixed(b)) => a == b,
            (Self::Action(a), Self::Action(b)) => a().partial_eq(&*b()),
            _ => false,
        }
    }
}

impl Eq for KeySpec {}

impl std::fmt::Debug for KeySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(k) => write!(f, "Fixed({k:?})"),
            Self::Action(a) => write!(f, "Action({})", a().name()),
        }
    }
}

/// A keystroke as caps: modifiers as symbols, then the key (`⌃` `⏎`).
pub fn key_caps(modifiers: &Modifiers, key: &str) -> Vec<SharedString> {
    let mut caps = Vec::with_capacity(3);
    for (on, cap) in [
        (modifiers.control, "⌃"),
        (modifiers.alt, "⌥"),
        (modifiers.shift, "⇧"),
        (modifiers.platform, "◆"),
    ] {
        if on {
            caps.push(SharedString::new_static(cap));
        }
    }
    caps.push(match key {
        "enter" => "⏎".into(),
        "escape" => "Esc".into(),
        "backspace" => "⌫".into(),
        "delete" => "Del".into(),
        "tab" => "Tab".into(),
        "space" => "Space".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        "left" => "←".into(),
        "right" => "→".into(),
        key => key.to_uppercase().into(),
    });
    caps
}

impl StatusItem {
    pub const BRAND: Self = Self::new(StatusKind::Brand);
    pub const MESSAGES: Self = Self::new(StatusKind::Messages);
    pub const MINIMIZED: Self = Self::new(StatusKind::Minimized);
    pub const CONTEXT_ACTIONS: Self = Self::new(StatusKind::ContextActions);
    pub const HINTS: Self = Self::new(StatusKind::Hints);

    const fn new(kind: StatusKind) -> Self {
        Self {
            kind,
            on_click: None,
        }
    }

    /// Static text (usable in statics).
    pub const fn label(text: &'static str) -> Self {
        Self::new(StatusKind::Text(SharedString::new_static(text)))
    }

    pub fn text(text: impl Into<SharedString>) -> Self {
        Self::new(StatusKind::Text(text.into()))
    }

    /// Key caps and a label (usable in statics).
    pub const fn keys(keys: KeySpec, label: &'static str) -> Self {
        Self::new(StatusKind::Keys {
            keys,
            label: SharedString::new_static(label),
        })
    }

    pub fn status(label: impl Into<SharedString>, status: BusyStatus) -> Self {
        Self::new(StatusKind::Status {
            label: label.into(),
            status,
        })
    }

    /// Runs `f` when clicked.
    pub const fn on_click(mut self, f: OnClick) -> Self {
        self.on_click = Some(f);
        self
    }
}

// ----- drawing ----------------------------------------------------------------------------------

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
pub fn status_dot(status: &BusyStatus, id: impl Into<ElementId>, theme: &ThemeData) -> AnyElement {
    let dot = div().flex_none().size(px(6.)).rounded_full();
    match status {
        BusyStatus::Ok => dot.bg(theme.color_succ).into_any_element(),
        BusyStatus::Err => dot.bg(theme.color_err).into_any_element(),
        BusyStatus::Busy => dot
            .bg(theme.accent)
            .with_animation(
                id,
                Animation::new(Duration::from_millis(1200))
                    .repeat()
                    .with_easing(pulsating_between(0.3, 1.0)),
                |el, t| el.opacity(t),
            )
            .into_any_element(),
    }
}

/// A view item; `None` for shell items and key hints, which the shell draws (it resolves their
/// keys). `id` keeps animations apart.
pub fn view_item(kind: &StatusKind, id: usize, theme: &ThemeData) -> Option<Div> {
    Some(match kind {
        StatusKind::Text(text) => div().child(text.clone()),
        StatusKind::Status { label, status } => div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(status_dot(
                status,
                ElementId::NamedInteger("status-bar-dot".into(), id as u64),
                theme,
            ))
            .child(label.clone()),
        StatusKind::Keys { .. }
        | StatusKind::Brand
        | StatusKind::Messages
        | StatusKind::Minimized
        | StatusKind::ContextActions
        | StatusKind::Hints => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static FIXED: [StatusItem; 3] = [
        StatusItem::keys(KeySpec::Fixed(&["Esc"]), "stop"),
        StatusItem::keys(KeySpec::action::<crate::ui::actions::Execute>(), "run"),
        StatusItem::label("hi"),
    ];

    #[test]
    fn key_specs_compare_by_what_they_press() {
        use crate::ui::actions::{Execute, Quit};
        assert_eq!(KeySpec::action::<Execute>(), KeySpec::action::<Execute>());
        assert_ne!(KeySpec::action::<Execute>(), KeySpec::action::<Quit>());
    }

    #[test]
    fn caps() {
        let ctrl = Modifiers {
            control: true,
            ..Default::default()
        };
        assert_eq!(key_caps(&ctrl, "enter"), ["⌃", "⏎"]);
        assert_eq!(key_caps(&Modifiers::default(), "c"), ["C"]);
    }

    #[test]
    fn default_borrows_statics() {
        let bar = StatusBar::default();
        assert!(matches!(bar.left, Cow::Borrowed(_)));
        assert!(matches!(bar.right, Cow::Borrowed(_)));
        let kinds: Vec<_> = bar.left.iter().map(|i| i.kind.clone()).collect();
        assert_eq!(
            kinds,
            [
                StatusKind::Brand,
                StatusKind::Messages,
                StatusKind::Minimized
            ]
        );
        assert_eq!(bar.right[0].kind, StatusKind::ContextActions);
    }

    #[test]
    fn fixed_bar_stays_borrowed() {
        let bar = StatusBar::shell().with_right(&FIXED[..]);
        assert!(matches!(bar.right, Cow::Borrowed(_)));
        assert_eq!(bar.right.len(), 3);
    }

    #[test]
    fn pushing_copies_once() {
        let bar = StatusBar::shell()
            .right(StatusItem::text("a"))
            .right(StatusItem::text("b"));
        assert!(matches!(bar.right, Cow::Owned(_)));
        assert_eq!(bar.right.len(), 2);
        assert_eq!(bar.left.len(), 3);
    }
}
