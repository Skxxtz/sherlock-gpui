use gpui::{Div, IntoElement, Keystroke, ParentElement, SharedString, Styled, div, px};
use std::fmt;

use crate::app::theme::ThemeData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mod {
    Ctrl,
    Alt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextActionBinds {
    pub modifier: Mod,
    pub key: char,
}

impl ContextActionBinds {
    pub fn from_keystroke(k: &Keystroke) -> Option<Self> {
        let m = &k.modifiers;
        let modifier = match (m.control, m.alt, m.shift, m.platform) {
            (true, false, false, false) => Mod::Ctrl,
            (false, true, false, false) => Mod::Alt,
            _ => return None,
        };
        let mut chars = k.key.chars();
        let key = chars.next()?;
        chars.next().is_none().then_some(Self { modifier, key })
    }

    fn keycap(content: impl IntoElement, theme: &ThemeData) -> Div {
        let size = px(18.);
        div()
            .flex()
            .items_center()
            .justify_center()
            .min_w(size)
            .h(size)
            .px(px(3.))
            .rounded(px(4.))
            .border(px(1.))
            .border_color(theme.border_idle)
            .bg(theme.bg_muted)
            .text_center()
            .line_height(size)
            .child(div().relative().top(px(0.5)).child(content))
    }

    pub fn hint(self, theme: &ThemeData) -> Div {
        // ⌃ sits high in its line
        let (symbol, nudge) = match self.modifier {
            Mod::Ctrl => ("⌃", px(1.5)),
            Mod::Alt => ("⌥", px(0.5)),
        };

        div()
            .flex_shrink_0()
            .ml_auto()
            .flex()
            .items_center()
            .gap(px(3.))
            .text_size(px(10.))
            .font_family(theme.font_family.clone())
            .text_color(theme.secondary_text)
            .child(Self::keycap(
                div().relative().top(nudge).child(symbol),
                theme,
            ))
            .child(Self::keycap(
                SharedString::from(self.key.to_ascii_uppercase().to_string()),
                theme,
            ))
    }
}

impl fmt::Display for ContextActionBinds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = match self.modifier {
            Mod::Ctrl => "⌃",
            Mod::Alt => "⌥",
        };
        write!(f, "{m}-{}", self.key)
    }
}

/// Mnemonic candidates, best first: word initials, then all letters, then digits;
/// ctrl before alt.
fn candidates(name: &str) -> impl Iterator<Item = ContextActionBinds> + '_ {
    let letters = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .chain(name.chars())
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .chain('1'..='9');

    [Mod::Ctrl, Mod::Alt].into_iter().flat_map(move |modifier| {
        letters
            .clone()
            .map(move |key| ContextActionBinds { modifier, key })
    })
}

pub fn assign_keys<'a>(
    names: impl IntoIterator<Item = &'a str>,
    reserved: &[ContextActionBinds],
) -> Vec<Option<ContextActionBinds>> {
    let mut taken: Vec<ContextActionBinds> = Vec::new();
    names
        .into_iter()
        .map(|name| {
            let key = candidates(name).find(|k| !reserved.contains(k) && !taken.contains(k));
            taken.extend(key);
            key
        })
        .collect()
}
