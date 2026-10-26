/// Selection over a list of ids.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    index: usize,
    /// Set by an explicit selection, cleared by `reset`. While set, `restore` keeps the same item
    /// selected if it is still visible.
    sticky: bool,
}

impl Cursor {
    /// Selected position, clamped to `len`
    #[inline]
    pub fn index(&self, len: usize) -> usize {
        self.index.min(len.saturating_sub(1))
    }

    /// Id at the selected position, if any.
    #[inline]
    pub fn id<Id: Copy>(&self, ids: &[Id]) -> Option<Id> {
        ids.get(self.index(ids.len())).copied()
    }

    /// Selects position `n`. Returns `false` (and changes nothing) if out of range.
    pub fn select(&mut self, n: usize, len: usize) -> bool {
        if n >= len {
            return false;
        }

        *self = Self {
            index: n,
            sticky: true,
        };
        true
    }

    /// Selects `id`. Returns `false` if it isn't in `ids`.
    pub fn select_id<Id: PartialEq>(&mut self, id: &Id, ids: &[Id]) -> bool {
        match ids.iter().position(|i| i == id) {
            Some(n) => self.select(n, ids.len()),
            None => false,
        }
    }

    /// Back to the first item; the next `restore` wont try to keep the selection.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Call when the visible ids change from `old` to `new`.
    /// Returns `true` if the previously selected item is still visible and stays selected.
    pub fn restore<Id: Copy + PartialEq>(&mut self, old: &[Id], new: &[Id]) -> bool {
        if !self.sticky {
            return false;
        }
        let Some(prev) = self.id(old) else {
            return false;
        };
        match new.iter().position(|&i| i == prev) {
            Some(n) => {
                self.index = n;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_clamped() {
        let mut c = Cursor::default();
        assert!(c.select(4, 5));
        assert_eq!(c.index(5), 4);
        assert_eq!(c.index(2), 1);
        assert_eq!(c.index(0), 0);
    }

    #[test]
    fn select_out_of_range_changes_nothing() {
        let mut c = Cursor::default();
        assert!(!c.select(3, 3));
        assert_eq!(c, Cursor::default());
    }

    #[test]
    fn id_of_empty_is_none() {
        assert_eq!(Cursor::default().id::<u32>(&[]), None);
    }

    #[test]
    fn select_id_finds_position() {
        let mut c = Cursor::default();
        assert!(c.select_id(&30, &[10, 20, 30]));
        assert_eq!(c.id(&[10, 20, 30]), Some(30));
        assert!(!c.select_id(&99, &[10, 20, 30]));
        assert_eq!(c.id(&[10, 20, 30]), Some(30));
    }

    #[test]
    fn restore_follows_item_to_new_position() {
        let mut c = Cursor::default();
        c.select(1, 3); // 20
        assert!(c.restore(&[10, 20, 30], &[30, 20]));
        assert_eq!(c.id(&[30, 20]), Some(20));
    }

    #[test]
    fn restore_fails_when_item_gone() {
        let mut c = Cursor::default();
        c.select(1, 3);
        assert!(!c.restore(&[10, 20, 30], &[10, 30]));
    }

    #[test]
    fn restore_ignored_when_not_sticky() {
        let mut c = Cursor::default();
        c.select(1, 3);
        c.reset();
        assert!(!c.restore(&[10, 20, 30], &[20, 10]));
        assert_eq!(c.index(2), 0);
    }
}
