use std::ops::Range;

use gpui::SharedString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSelection {
    /// Render-order id of the text block.
    pub block: usize,
    /// The block's plain text, as rendered.
    pub text: SharedString,
    /// Where the drag started and where it is now.
    pub anchor: usize,
    pub head: usize,
    /// 1 = characters, 2 = words, 3 = the whole block.
    pub clicks: usize,
    pub dragging: bool,
}

impl TextSelection {
    pub fn start(block: usize, text: SharedString, at: usize, clicks: usize) -> Self {
        let at = floor_char(&text, at);
        let mut sel = Self {
            block,
            text,
            anchor: at,
            head: at,
            clicks: clicks.clamp(1, 3),
            dragging: true,
        };
        sel.extend_to(at);
        sel
    }

    /// Moves the head to `at`, snapping to words or the whole block for double/triple clicks.
    pub fn extend_to(&mut self, at: usize) {
        self.head = floor_char(&self.text, at);
    }

    /// The selected byte range, snapped to the click granularity.
    pub fn range(&self) -> Range<usize> {
        let (a, b) = (self.anchor.min(self.head), self.anchor.max(self.head));
        match self.clicks {
            1 => a..b,
            2 => word_at(&self.text, a).start..word_at(&self.text, b).end,
            _ => 0..self.text.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.range().is_empty()
    }

    pub fn selected_text(&self) -> &str {
        &self.text[self.range()]
    }
}

/// The word around byte `at`: letters, digits and `_-./:@~` (so paths, URLs, IPs and flags
/// select whole), or the single other character there.
pub fn word_at(text: &str, at: usize) -> Range<usize> {
    let at = floor_char(text, at.min(text.len()));
    let is_word = |c: char| c.is_alphanumeric() || "_-./:@~".contains(c);
    let start = text[..at]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| is_word(c))
        .last()
        .map_or(at, |(i, _)| i);
    let end = text[at..]
        .char_indices()
        .take_while(|&(_, c)| is_word(c))
        .last()
        .map_or(at, |(i, c)| at + i + c.len_utf8());
    if start == end && at < text.len() {
        // Not on a word: select the one character.
        let len = text[at..].chars().next().map_or(0, char::len_utf8);
        return at..at + len;
    }
    start..end
}

fn floor_char(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(text: &'static str, from: usize, to: usize, clicks: usize) -> String {
        let mut s = TextSelection::start(0, text.into(), from, clicks);
        s.extend_to(to);
        s.selected_text().to_owned()
    }

    #[test]
    fn drag_selects_characters_either_direction() {
        assert_eq!(sel("hello world", 0, 5, 1), "hello");
        assert_eq!(sel("hello world", 11, 6, 1), "world");
    }

    #[test]
    fn double_click_selects_whole_tokens() {
        assert_eq!(sel("run sudo ss -ltnp now", 13, 13, 2), "-ltnp");
        assert_eq!(
            sel("go to https://zed.dev/docs please", 10, 10, 2),
            "https://zed.dev/docs"
        );
        assert_eq!(sel("ip 192.168.178.1 here", 5, 5, 2), "192.168.178.1");
        // dragging after a double click extends by words
        assert_eq!(sel("one two three", 1, 9, 2), "one two three");
    }

    #[test]
    fn triple_click_selects_block() {
        assert_eq!(sel("all of it", 3, 3, 3), "all of it");
    }

    #[test]
    fn utf8_safe() {
        assert_eq!(sel("größe ändern", 3, 3, 2), "größe");
        assert_eq!(word_at("a — b", 2), 2..5, "a non-word char selects itself");
    }
}
