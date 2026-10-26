use gpui::{App, ScrollHandle};
use scroll::SidebarScroll;

use crate::ui::{
    launcher::navigator::{Navigator, handle::ViewKind},
    model::item::SelectionKey,
};

mod scroll;

#[derive(Default)]
pub struct SidebarController {
    for_item: Option<SelectionKey>,
    scroll: SidebarScroll,
    valid: bool,
}

impl SidebarController {
    pub fn check_validity(&mut self, nav: &Navigator, cx: &App) {
        // sidebar has a foritem
        if self.for_item.is_none() {
            self.valid = false;
            return;
        }

        // current view allows sidebar
        if !matches!(nav.current_kind(), ViewKind::Home | ViewKind::Files { .. }) {
            self.valid = false;
            return;
        }

        // current index is the same as for item
        if nav.selected_key(cx) != self.for_item {
            self.for_item = None;
            self.valid = false;
            return;
        }

        self.valid = true;
    }

    pub fn set_for_item(&mut self, item: SelectionKey) {
        self.for_item = Some(item);
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Restores the remembered sidebar scroll position
    pub fn sync_scroll(&mut self) {
        self.scroll.sync();
    }

    /// Scroll handle to attatch to the sidebar
    pub fn scroll_handle(&self) -> &ScrollHandle {
        &self.scroll.handle
    }
}
