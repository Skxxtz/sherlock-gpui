#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    /// Nothing: Enter sends the search text.
    #[default]
    None,
    /// A runnable item (code block or action) of the shown answer.
    Item(usize),
    /// The output panel of an item that was run.
    Output(usize),
    /// A suggested follow-up question of the shown answer.
    Suggestion(usize),
    /// A recent question on the empty view.
    Recent(usize),
}

impl Focus {
    /// The item that's focused, or whose output is.
    pub fn item(self) -> Option<usize> {
        match self {
            Self::Item(i) | Self::Output(i) => Some(i),
            _ => None,
        }
    }

    /// The item itself, not its output.
    pub fn selected_item(self) -> Option<usize> {
        match self {
            Self::Item(i) => Some(i),
            _ => None,
        }
    }

    pub fn output(self) -> Option<usize> {
        match self {
            Self::Output(i) => Some(i),
            _ => None,
        }
    }

    pub fn suggestion(self) -> Option<usize> {
        match self {
            Self::Suggestion(i) => Some(i),
            _ => None,
        }
    }

    pub fn recent(self) -> Option<usize> {
        match self {
            Self::Recent(i) => Some(i),
            _ => None,
        }
    }

    pub fn is_none(self) -> bool {
        self == Self::None
    }
}

/// An answer's focusable elements
pub fn stops(
    items: usize,
    has_output: impl Fn(usize) -> bool + Copy,
    suggestions: usize,
) -> impl Iterator<Item = Focus> + Clone {
    (0..items)
        .flat_map(move |i| {
            std::iter::once(Focus::Item(i)).chain(has_output(i).then_some(Focus::Output(i)))
        })
        .chain((0..suggestions).map(Focus::Suggestion))
}

/// Move the focus. On overflow focuses none.
pub fn cycle(current: Focus, stops: impl Iterator<Item = Focus>, forward: bool) -> Focus {
    let mut prev = None;
    let mut first = None;
    let mut found = false;
    for s in stops {
        if forward {
            if found {
                return s;
            }
            first.get_or_insert(s);
            found = s == current;
        } else {
            if s == current {
                return prev.unwrap_or(Focus::None);
            }
            prev = Some(s);
        }
    }
    match (forward, found) {
        (true, true) => Focus::None, // was the last stop
        (true, false) => first.unwrap_or(Focus::None),
        (false, _) => prev.unwrap_or(Focus::None), // not on a stop: the last one
    }
}

/// Move the focus within the range of its focus stops. No overflow
pub fn down(current: Focus, stops: impl Iterator<Item = Focus>) -> Focus {
    match cycle(current, stops, true) {
        Focus::None => current,
        next => next,
    }
}

/// The next recent question, staying on the last.
pub fn recent_down(current: Focus, count: usize) -> Focus {
    if count == 0 {
        return Focus::None;
    }
    Focus::Recent(current.recent().map_or(0, |i| (i + 1).min(count - 1)))
}

/// The previous recent question; off the top, nothing.
pub fn recent_up(current: Focus) -> Focus {
    current
        .recent()
        .and_then(|i| i.checked_sub(1))
        .map_or(Focus::None, Focus::Recent)
}

#[cfg(test)]
mod tests {
    use super::{Focus::*, *};

    #[test]
    fn stops_put_outputs_after_their_item() {
        assert_eq!(
            stops(3, |i| i == 1, 0).collect::<Vec<_>>(),
            [Item(0), Item(1), Output(1), Item(2)]
        );
        assert_eq!(stops(0, |_| true, 0).next(), Option::None);
        assert_eq!(
            stops(1, |_| false, 2).collect::<Vec<_>>(),
            [Item(0), Suggestion(0), Suggestion(1)]
        );
    }

    #[test]
    fn cycle_walks_and_falls_off_the_ends() {
        let s = stops(2, |i| i == 0, 0);
        assert_eq!(cycle(None, s.clone(), true), Item(0));
        assert_eq!(cycle(Item(0), s.clone(), true), Output(0));
        assert_eq!(cycle(Output(0), s.clone(), true), Item(1));
        assert_eq!(cycle(Item(1), s.clone(), true), None);
        assert_eq!(cycle(None, s.clone(), false), Item(1));
        assert_eq!(cycle(Item(0), s, false), None);
        assert_eq!(cycle(None, std::iter::empty(), true), None);
    }

    #[test]
    fn a_focus_that_no_longer_exists_restarts() {
        // e.g. the output was cleared
        let s = stops(2, |_| false, 0);
        assert_eq!(cycle(Output(0), s, true), Item(0));
    }

    #[test]
    fn down_stays_on_the_last_stop() {
        let s = stops(1, |_| true, 0);
        assert_eq!(down(None, s.clone()), Item(0));
        assert_eq!(down(Item(0), s.clone()), Output(0));
        assert_eq!(down(Output(0), s), Output(0));
    }

    #[test]
    fn recent_questions() {
        assert_eq!(recent_down(None, 3), Recent(0));
        assert_eq!(recent_down(Recent(2), 3), Recent(2));
        assert_eq!(recent_down(None, 0), None);
        assert_eq!(recent_up(Recent(1)), Recent(0));
        assert_eq!(recent_up(Recent(0)), None);
    }

    #[test]
    fn accessors() {
        assert_eq!(Output(2).item(), Some(2));
        assert_eq!(Output(2).selected_item(), Option::None);
        assert_eq!(Item(1).selected_item(), Some(1));
        assert!(Focus::default().is_none());
    }
}
