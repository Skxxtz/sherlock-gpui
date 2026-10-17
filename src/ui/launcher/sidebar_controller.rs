use gpui::App;

use crate::{
    launcher::LauncherId,
    ui::launcher::navigator::{Navigator, handle::ViewKind},
};

#[derive(Default)]
pub struct SidebarController {
    for_item: Option<(LauncherId, usize)>,
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
        let current_data_idx = nav.selected_data_idx(cx);
        if current_data_idx != self.for_item {
            self.for_item = None;
            self.valid = false;
            return;
        }

        self.valid = true;
    }

    pub fn set_for_item(&mut self, item: (LauncherId, usize)) {
        self.for_item = Some(item);
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn launcher(&self) -> Option<LauncherId> {
        self.for_item.map(|(id, _)| id)
    }
}
