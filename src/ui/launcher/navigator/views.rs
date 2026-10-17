use gpui::{ListState, Pixels, UniformListScrollHandle};

use crate::ui::launcher::{
    LauncherMode,
    navigator::handle::{ViewHandle, ViewKind},
};

mod emoji;
mod file;
mod home;
mod message;
mod process;

/// One entry in the navigator: a view plus its selection and scroll state.
pub struct NavView {
    pub handle: ViewHandle,
    pub layout: Layout,
    pub kind: ViewKind,
    pub mode: LauncherMode,
}

impl NavView {
    /// Shared constructor for the per-view builders. Starts in `LauncherMode::Home`.
    fn new(handle: ViewHandle, layout: Layout, kind: ViewKind) -> Self {
        Self {
            handle,
            layout,
            kind,
            mode: LauncherMode::Home,
        }
    }
}

/// Selection and scroll state. `grid: None` means a plain row list.
#[derive(Clone)]
pub struct Layout {
    /// Scroll state for row lists
    pub list: ListState,
    /// Selected position in the filtered results (not clamped).
    pub selected: usize,
    /// Set for grid layouts.
    pub grid: Option<Grid>,
}

/// Extra state for grid layouts.
#[derive(Clone)]
pub struct Grid {
    /// Items per row.
    pub columns: usize,
    /// Scroll handle for the grid's rows.
    pub scroll: UniformListScrollHandle,
}

/// Direction for keyboard navigation.
/// `Left/Right` only move in grid layouts.
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Layout {
    /// A plain vertical list.
    /// `len` is the initial item count, `item_height` the estimated row height.
    pub fn row(len: usize, item_height: Pixels) -> Self {
        Self {
            list: ListState::new(len, gpui::ListAlignment::Top, item_height),
            selected: 0,
            grid: None,
        }
    }

    ///  A grid with `columns` items per row (at least 1).
    ///  Arrow keys move in all four directions.
    pub fn grid(len: usize, item_height: Pixels, columns: usize) -> Self {
        Self {
            grid: Some(Grid {
                columns: columns.max(1),
                scroll: UniformListScrollHandle::new(),
            }),
            ..Self::row(len, item_height)
        }
    }

    /// Resets the selected index.
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    /// Returns `(current, new)`. Left/Right only move in grids.
    pub fn next_index(&self, dir: MoveDirection) -> Option<(usize, usize)> {
        let cur = self.selected;
        let step = self.grid.as_ref().map_or(1, |g| g.columns);
        let is_grid = self.grid.is_some();

        let new = match dir {
            MoveDirection::Down => cur.checked_add(step),
            MoveDirection::Up => cur.checked_sub(step),
            MoveDirection::Right if is_grid => cur.checked_add(1),
            MoveDirection::Left if is_grid => cur.checked_sub(1),
            _ => None,
        }?;

        Some((cur, new))
    }

    /// Selected item `n` and scrolls it into view.
    /// In a grid, it scrolls to the row containing `n`.
    pub fn focus_nth(&mut self, n: usize) {
        self.selected = n;
        match &self.grid {
            Some(g) => g
                .scroll
                .scroll_to_item(n / g.columns, gpui::ScrollStrategy::Nearest),
            None => self.list.scroll_to_reveal_item(n),
        }
    }
}
