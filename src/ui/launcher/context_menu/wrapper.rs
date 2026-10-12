use std::{sync::Arc, time::Duration};

use gpui::{
    Animation, AnimationExt, InteractiveElement, IntoElement, ParentElement, ScrollHandle,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherValues,
        emoji_launcher::{ALL_SKIN_TONES, SkinTone},
        variant_type::QuickActionCapabilities,
    },
    ui::{
        launcher::{
            context_menu::{ContextMenuAction, quick_actions::QuickAction},
            views::MoveDirection,
        },
        widgets::{
            RenderableChild,
            emoji::{get_selected_skin_tones, set_all_skin_tones, set_selected_skin_tone},
        },
    },
};

pub struct QuickActionList {
    actions: Arc<[Arc<ContextMenuAction>]>,
    /// bitmask; i = action i is active
    active: QuickActionCapabilities,
}

impl QuickActionList {
    fn is_active(&self, action: &ContextMenuAction) -> bool {
        match action {
            ContextMenuAction::QuickAction(q) => self.active.contains(q.requires()),
            _ => true,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<ContextMenuAction>> + '_ {
        self.actions.iter().filter(|a| self.is_active(a))
    }

    pub fn len(&self) -> usize {
        self.active.bits().count_ones() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_mask(&mut self, mask: QuickActionCapabilities) {
        self.active = mask;
    }

    fn mask(&self) -> u8 {
        if self.actions.len() >= 8 {
            u8::MAX
        } else {
            (1 << self.actions.len()) - 1
        }
    }
}

impl Default for QuickActionList {
    fn default() -> Self {
        Self {
            actions: Arc::new([Arc::new(ContextMenuAction::QuickAction(QuickAction::Hide))]),
            active: QuickActionCapabilities::empty(),
        }
    }
}

pub struct ContextMenu {
    pub idx: Option<usize>,

    pub actions: Arc<[Arc<ContextMenuAction>]>,
    pub quick_actions: QuickActionList,

    pub has_actions: bool,

    scroll_handle: ScrollHandle,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            idx: None,
            actions: Arc::new([]),
            quick_actions: QuickActionList::default(),
            has_actions: true,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

pub trait Move {
    fn down(&mut self);
    fn up(&mut self);
    fn left(&mut self);
    fn right(&mut self);
    fn handle_move(&mut self, direction: MoveDirection) {
        match direction {
            MoveDirection::Down => self.down(),
            MoveDirection::Up => self.up(),
            MoveDirection::Left => self.left(),
            MoveDirection::Right => self.right(),
        }
    }
}

impl Move for ContextMenu {
    fn down(&mut self) {
        let n = self.num_actions();
        if n == 0 {
            return;
        }
        self.idx = Some(match self.idx {
            Some(i) => (i + 1).min(n - 1),
            None => 0,
        });
        self.scroll_selected_into_view();
    }
    fn up(&mut self) {
        if let Some(idx) = self.idx
            && idx > 0
        {
            self.idx = Some(idx - 1);
        }
        self.scroll_selected_into_view();
    }
    fn left(&mut self) {
        if let Some(action_arc) = self.get_current() {
            if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                act.update_index(|i| {
                    let new = i.saturating_sub(1).min(5);
                    set_selected_skin_tone(new.into(), act.for_tone as usize);
                    new
                });

                if get_selected_skin_tones()
                    .iter()
                    .any(|t| matches!(t, SkinTone::Simpsons))
                {
                    set_all_skin_tones(SkinTone::Simpsons);
                    self.actions.iter().for_each(|action| {
                        if let ContextMenuAction::Emoji(act) = action.as_ref() {
                            act.update_index(|_| 0)
                        }
                    });
                }
                self.scroll_selected_into_view();
            } else {
                self.up();
            }
        }
    }
    fn right(&mut self) {
        if let Some(action_arc) = self.get_current() {
            if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                // Counts the number of `SkinTone::Simpsons` occurrences in the selected
                // skintones.
                let num_simpsons = || {
                    get_selected_skin_tones()
                        .iter()
                        .filter(|t| matches!(t, SkinTone::Simpsons))
                        .count()
                };
                let before = num_simpsons();

                // update index
                act.update_index(|i| {
                    let new = (i + 1).clamp(0, 5);
                    set_selected_skin_tone(new.into(), act.for_tone as usize);
                    new
                });

                // identify change away from "all-simpsons" state
                if before > num_simpsons() {
                    set_all_skin_tones(ALL_SKIN_TONES[1]);
                    self.actions.iter().for_each(|action| {
                        if let ContextMenuAction::Emoji(act) = action.as_ref() {
                            act.update_index(|_| 1)
                        }
                    });
                }
                self.scroll_selected_into_view();
            } else {
                self.down();
            }
        }
    }
}

impl ContextMenu {
    pub fn is_open(&self) -> bool {
        self.idx.is_some()
    }

    pub fn close(&mut self) -> bool {
        self.idx.take().is_some()
    }

    pub fn open(&mut self, idx: usize) {
        self.idx = Some(idx);
    }

    pub fn can_open(&self) -> bool {
        self.has_actions
    }

    pub fn num_actions(&self) -> usize {
        self.actions.len().saturating_add(self.quick_actions.len())
    }

    pub fn set_actions(&mut self, actions: Arc<[Arc<ContextMenuAction>]>) {
        self.actions = actions;
    }

    pub fn clear_actions(&mut self) {
        self.set_actions(Default::default());
    }

    pub fn availablilty_mask(itm: &RenderableChild) -> QuickActionCapabilities {
        itm.launcher_type().capabilities()
    }

    pub fn get(&self, index: usize) -> Option<&Arc<ContextMenuAction>> {
        let groups: [&[Arc<ContextMenuAction>]; 2] = [&self.actions, &self.quick_actions.actions];
        let mut index = index;
        for g in groups {
            if index < g.len() {
                return Some(&g[index]);
            }
            index -= g.len();
        }
        None
    }

    pub fn get_current(&self) -> Option<&Arc<ContextMenuAction>> {
        self.idx.and_then(|idx| self.get(idx))
    }
}

impl ContextMenu {
    fn scroll_selected_into_view(&self) {
        let Some(idx) = self.idx else { return };
        let child_index = if idx < self.actions.len() {
            idx
        } else {
            idx + 1
        };
        self.scroll_handle.scroll_to_item(child_index);
    }
    pub fn focus_first(&mut self) {
        if self.can_open() {
            self.idx = Some(0);
            self.scroll_selected_into_view();
        }
    }
    pub fn render(&self, theme: Arc<ThemeData>) -> impl IntoElement {
        if let Some(active) = self.idx {
            let render = |child: &Arc<ContextMenuAction>, is_selected: bool| match child.as_ref() {
                ContextMenuAction::Emoji(_) => child
                    .render_emoji_col(is_selected, theme.clone())
                    .into_any_element(),
                _ => child
                    .render_row(is_selected, theme.clone())
                    .into_any_element(),
            };
            let offset = self.actions.len();

            let separator = div()
                .h(px(1.))
                .mx(px(4.))
                .my(px(3.))
                .flex_shrink_0()
                .bg(theme.border_idle);

            div().inset_0().absolute().child(
                div()
                    .p(px(7.))
                    .bg(theme.bg_app)
                    .border_color(theme.border)
                    .border(px(1.))
                    .rounded_md()
                    .absolute()
                    .bottom(px(10.))
                    .right(px(10.))
                    .min_w(px(200.))
                    .max_w(px(500.))
                    .child(
                        div()
                            .id("context-menu-scroll")
                            .track_scroll(&self.scroll_handle)
                            .max_h(px(500.))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .children(
                                self.actions
                                    .iter()
                                    .enumerate()
                                    .map(|(i, c)| render(c, i == active)),
                            )
                            .when(
                                !self.actions.is_empty() && !self.quick_actions.is_empty(),
                                |this| this.child(separator),
                            )
                            .children(
                                self.quick_actions
                                    .iter()
                                    .enumerate()
                                    .map(|(i, c)| render(c, i + offset == active)),
                            ),
                    )
                    .with_animation(
                        "context-reveal",
                        Animation::new(Duration::from_millis(120)).with_easing(gpui::ease_in_out),
                        |this, delta| this.opacity(delta).occlude(),
                    ),
            )
        } else {
            div()
        }
    }
}
