use gpui::{App, ListState, Pixels, UniformListScrollHandle};

use crate::ui::launcher::{LauncherMode, navigator::handle::ViewHandle};

/// One entry in the navigator: a view plus its scroll state and mode.
pub struct NavView {
    pub handle: ViewHandle,
    pub layout: Layout,
    pub mode: LauncherMode,
}

impl NavView {
    /// Entry for `view`, with the layout the view asks for (`SherlockView::layout`).
    /// Starts in `LauncherMode::Home`.
    pub fn new(view: impl Into<ViewHandle>, cx: &App) -> Self {
        let handle = view.into();
        let layout = handle.layout(cx);
        Self::with_layout(handle, layout)
    }

    /// Entry for `view` with an explicit layout.
    pub fn with_layout(view: impl Into<ViewHandle>, layout: Layout) -> Self {
        Self {
            handle: view.into(),
            layout,
            mode: LauncherMode::Home,
        }
    }
}

/// Scroll state. `grid: None` means a plain row list.
/// The selection itself lives in the view's `Model`.
#[derive(Clone)]
pub struct Layout {
    /// Scroll state for row lists
    pub list: ListState,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// The position one step from `cur` in `dir`. Left/Right only move in grids.
    pub fn next_index(&self, cur: usize, dir: MoveDirection) -> Option<usize> {
        let step = self.grid.as_ref().map_or(1, |g| g.columns);
        let is_grid = self.grid.is_some();

        match dir {
            MoveDirection::Down => cur.checked_add(step),
            MoveDirection::Up => cur.checked_sub(step),
            MoveDirection::Right if is_grid => cur.checked_add(1),
            MoveDirection::Left if is_grid => cur.checked_sub(1),
            _ => None,
        }
    }

    /// Scrolls item `n` into view. In a grid, scrolls to the row containing `n`.
    pub fn reveal(&self, n: usize) {
        match &self.grid {
            Some(g) => g
                .scroll
                .scroll_to_item(n / g.columns, gpui::ScrollStrategy::Nearest),
            None => self.list.scroll_to_reveal_item(n),
        }
    }
}
