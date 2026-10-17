use std::{sync::Arc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, InteractiveElement, IntoElement, Keystroke, ParentElement,
    ScrollHandle, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};

use crate::{
    app::{bindings::ReservedKeys, theme::ThemeData},
    launcher::{
        LauncherId, LauncherValues,
        emoji_launcher::{ALL_SKIN_TONES, SkinTone},
        variant_type::QuickActionCapabilities,
    },
    ui::{
        launcher::{
            context_menu::{
                ContextMenuAction,
                action_keys::{ContextActionBinds, assign_keys},
                quick_actions::{Hideable, QuickAction},
            },
            navigator::views::MoveDirection,
        },
        traits::RenderableChildDelegate,
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
}

impl Default for QuickActionList {
    fn default() -> Self {
        Self {
            actions: Arc::new([
                Arc::new(ContextMenuAction::QuickAction(QuickAction::Hide)),
                Arc::new(ContextMenuAction::QuickAction(QuickAction::Info)),
            ]),
            active: QuickActionCapabilities::empty(),
        }
    }
}

pub struct ContextMenu {
    pub idx: Option<usize>,

    actions: Arc<[Arc<ContextMenuAction>]>,
    quick_actions: QuickActionList,

    pub keys: Vec<Option<ContextActionBinds>>,

    pub has_actions: bool,

    cached_for: Option<(LauncherId, usize)>,

    is_flashing: Option<usize>,
    scroll_handle: ScrollHandle,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            idx: None,
            actions: Arc::new([]),
            quick_actions: QuickActionList::default(),
            keys: Vec::new(),
            has_actions: true,
            cached_for: None,
            is_flashing: None,
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

    pub fn set_actions(&mut self, actions: Arc<[Arc<ContextMenuAction>]>, cx: &mut App) {
        self.actions = actions;
        let reserved = &cx.global::<ReservedKeys>();
        self.reassign_keys(reserved);
    }

    pub fn clear_actions(&mut self) {
        self.actions = Default::default();
        self.keys.clear();
    }

    pub fn availability_mask(itm: &RenderableChild) -> QuickActionCapabilities {
        let mut caps = itm.launcher_type().capabilities();
        if !itm.is_hideable() {
            caps.remove(QuickActionCapabilities::HIDE);
        }

        if !itm.has_sidebar() {
            caps.remove(QuickActionCapabilities::INFO);
        }

        // TODO: we could make a check for config values to allow users to allow/disallow certain
        // quickactions from the config files like: QuickActions: `["hide", "info", ..]`

        caps
    }

    pub fn refresh_for(
        &mut self,
        selected: Option<(LauncherId, usize)>,
        compute: impl FnOnce() -> (QuickActionCapabilities, bool),
    ) -> bool {
        if self.cached_for != selected {
            let (mask, has_actions) = compute();
            self.quick_actions.set_mask(mask);
            self.has_actions = has_actions || !self.quick_actions.is_empty();
            self.cached_for = selected;
        }
        self.has_actions
    }

    pub fn invalidate(&mut self) {
        self.cached_for = None;
    }

    fn reassign_keys(&mut self, reserved: &ReservedKeys) {
        let names = self
            .quick_actions
            .iter()
            .chain(self.actions.iter())
            .map(|a| a.name().unwrap_or(""));
        let mut keys = assign_keys(names, &reserved.0);

        keys.rotate_left(self.quick_actions.len());
        self.keys = keys;
    }

    pub fn action_index_for(&self, keystroke: &Keystroke) -> Option<usize> {
        let combo = ContextActionBinds::from_keystroke(keystroke)?;
        self.keys.iter().position(|k| *k == Some(combo))
    }

    pub fn flash(&mut self, idx: usize) {
        self.is_flashing = Some(idx);
    }

    pub fn clear_flash(&mut self) {
        self.is_flashing = None;
    }

    pub fn is_flashing(&mut self) -> bool {
        self.is_flashing.is_some()
    }

    pub fn get(&self, index: usize) -> Option<&Arc<ContextMenuAction>> {
        if let Some(action) = self.actions.get(index) {
            return Some(action);
        }
        self.quick_actions.iter().nth(index - self.actions.len())
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
        let Some(active) = self.idx else { return div() };

        let render = |i: usize, child: &Arc<ContextMenuAction>| {
            let is_selected = i == active;
            let is_flashing = self.is_flashing == Some(i);
            match child.as_ref() {
                ContextMenuAction::Emoji(_) => child
                    .render_emoji_col(is_selected, theme.clone())
                    .into_any_element(),
                _ => child
                    .render_row(
                        is_selected,
                        is_flashing,
                        theme.clone(),
                        self.keys.get(i).copied().flatten(),
                    )
                    .into_any_element(),
            }
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
                .min_w(px(300.))
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
                        .children(self.actions.iter().enumerate().map(|(i, c)| render(i, c)))
                        .when(
                            !self.actions.is_empty() && !self.quick_actions.is_empty(),
                            |this| this.child(separator),
                        )
                        .children(
                            self.quick_actions
                                .iter()
                                .enumerate()
                                .map(|(i, c)| render(i + offset, c)),
                        ),
                )
                .with_animation(
                    "context-reveal",
                    Animation::new(Duration::from_millis(120)).with_easing(gpui::ease_in_out),
                    |this, delta| this.opacity(delta).occlude(),
                ),
        )
    }
}
