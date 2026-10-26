use std::sync::Arc;

use gpui::{SharedString, Styled, prelude::FluentBuilder};

use crate::{app::theme::ThemeData, ui::utils::selection::Selection};

#[allow(dead_code)]
pub trait ListItemBorder: Styled + FluentBuilder + Sized {
    fn list_item_border(self, theme: &Arc<ThemeData>, selection: &Selection) -> Self {
        self.list_item_border_for(theme, selection.is_selected)
    }
    fn list_item_border_for(self, theme: &Arc<ThemeData>, selected: bool) -> Self {
        self.bg(theme.bg_idle)
            .rounded_md()
            .border_1()
            .when(selected, |this| {
                this.bg(theme.bg_selected)
                    .border_color(theme.border_selected)
            })
    }
    fn selected_text(self, theme: &Arc<ThemeData>, selection: &Selection) -> Self {
        self.text_color(theme.secondary_text)
            .when(selection.is_selected, |this| {
                this.text_color(theme.primary_text)
            })
    }
    fn tile_bg(self, theme: &Arc<ThemeData>, selection: &Selection) -> Self {
        self.bg(theme.bg_idle)
            .when(selection.is_selected, |this| this.bg(theme.bg_selected))
    }
}

impl<T: Styled + FluentBuilder + Sized> ListItemBorder for T {}

/// Replaces all occurrences of `{name}` in `to` with `value`.
///
/// Returns the original `SharedString` unchanged if no match is found,
/// avoiding any allocation. Only allocates on a successful match.
pub fn substitute(to: SharedString, name: &str, value: &str) -> SharedString {
    let s = to.as_ref();

    let mut i = 0;
    let found = loop {
        let Some(open) = s[i..].find('{') else {
            break false;
        };
        let start = i + open + 1;
        if s[start..].starts_with(name)
            && s.get(start + name.len()..)
                .is_some_and(|r| r.starts_with('}'))
        {
            break true;
        }
        i = start;
    };

    if found {
        let pattern = format!("{{{}}}", name);
        SharedString::from(s.replace(&pattern, value))
    } else {
        to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_single_occurrence() {
        let result = substitute("nordvpn -c {location}".into(), "location", "Germany");
        assert_eq!(result.as_ref(), "nordvpn -c Germany");
    }

    #[test]
    fn replaces_multiple_occurrences() {
        let result = substitute("echo {x} and {x}".into(), "x", "hello");
        assert_eq!(result.as_ref(), "echo hello and hello");
    }

    #[test]
    fn no_match_returns_original() {
        let original: SharedString = "nordvpn -c {location}".into();
        let result = substitute(original.clone(), "country", "Germany");
        assert_eq!(result, original);
    }

    #[test]
    fn partial_match_no_closing_brace() {
        // malformed `{location` — no `}`, should not match
        let original: SharedString = "nordvpn -c {location".into();
        let result = substitute(original.clone(), "location", "Germany");
        assert_eq!(result, original);
    }

    #[test]
    fn partial_match_wrong_name() {
        // `{locations}` should not match `location`
        let original: SharedString = "nordvpn -c {locations}".into();
        let result = substitute(original.clone(), "location", "Germany");
        assert_eq!(result, original);
    }

    #[test]
    fn empty_value() {
        let result = substitute("nordvpn -c {location}".into(), "location", "");
        assert_eq!(result.as_ref(), "nordvpn -c ");
    }

    #[test]
    fn empty_string() {
        let original: SharedString = "".into();
        let result = substitute(original.clone(), "location", "Germany");
        assert_eq!(result, original);
    }
}
