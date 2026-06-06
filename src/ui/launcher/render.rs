use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, Context, Element, Entity, FontWeight,
    InteractiveElement, IntoElement, MouseDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, deferred, div, img, list, prelude::FluentBuilder,
    px, relative,
};
use std::{sync::Arc, time::Duration};

use crate::{
    CONTEXT_MENU_BIND,
    app::{
        bindings::ShortcutKeyMod,
        theme::{ActiveTheme, ThemeData},
    },
    launcher::LauncherValues,
    loader::resolve_icon_path,
    ui::{
        UIFunction,
        launcher::{
            Execute, LauncherMode, LauncherView, context_menu::ContextMenuAction,
            views::EntityStyle,
        },
        traits::RenderableChildDelegate,
        utils::{ease::Ease, render::ListItemBorder, selection::Selection},
        widgets::RenderableChild,
    },
    utils::config::ConfigGuard,
};

impl Render for LauncherView {
    fn render(&mut self, _win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();

        self.has_actions = self
            .navigation
            .with_selected_item(cx, |itm, cx| itm.has_actions(cx))
            .unwrap_or(false);

        div()
            .id("sherlock")
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg_app)
            .border_2()
            .border_color(theme.border)
            .rounded(px(5.))
            .overflow_hidden()
            .on_action(cx.listener(Self::selection_up))
            .on_action(cx.listener(Self::selection_down))
            .on_action(cx.listener(Self::selection_left))
            .on_action(cx.listener(Self::selection_right))
            .on_action(cx.listener(Self::next_var))
            .on_action(cx.listener(Self::prev_var))
            .on_action(cx.listener(Self::execute_listener))
            .on_action(cx.listener(Self::execute_inplace_listener))
            .on_action(cx.listener(Self::shortcut_listener))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::open_context))
            .on_key_up(cx.listener(Self::inner_bind_listener))
            .child(self.render_search_bar(theme.clone()))
            .when(!self.config_initialized, |this| {
                this.child(self.render_config_banner(theme.clone()))
            })
            .child(self.render_mode_label(theme.clone()))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(match self.navigation.current().style {
                        EntityStyle::Grid { .. } => self
                            .render_result_grid(cx, theme.clone())
                            .into_any_element(),
                        EntityStyle::Row { .. } => {
                            self.render_results(cx, theme.clone()).into_any_element()
                        }
                    }),
            )
            .child(self.render_status_bar(theme, cx))
    }
}

impl LauncherView {
    fn render_search_bar(&self, theme: Arc<ThemeData>) -> impl IntoElement {
        let search_icon = ConfigGuard::read().ok().map(|c| c.search_bar_icon.clone());
        let is_search_mode = self.navigation.current_mode() == &LauncherMode::Search;

        // Extract everything upfront — avoid repeated unwraps inside closures
        let icon_enabled = search_icon.as_ref().is_some_and(|i| i.enable);
        let icon_path = search_icon.as_ref().map(|i| i.icon.clone());
        let icon_size = search_icon
            .as_ref()
            .map(|i| i.icon_size as f32)
            .unwrap_or(24.);
        let icon_back_path = search_icon.as_ref().map(|i| i.icon_back.clone());
        let icon_back_size = search_icon
            .as_ref()
            .map(|i| i.icon_back_size as f32)
            .unwrap_or(24.);

        div()
            .flex()
            .flex_row()
            .w_full()
            .items_center()
            .px_4()
            .py(px(4.))
            .gap_3()
            .when(icon_enabled, |this| {
                this.child(
                    div()
                        .flex()
                        .size(px(24.))
                        .items_center()
                        .justify_center()
                        .when(!is_search_mode, |this| {
                            this.when_some(
                                icon_path.as_deref().and_then(resolve_icon_path),
                                |this, icon| this.child(img(icon).size(px(icon_size))),
                            )
                        })
                        .when(is_search_mode, |this| {
                            this.when_some(
                                icon_back_path.as_deref().and_then(resolve_icon_path),
                                |this, icon| {
                                    this.child(if let Some(svg) = icon.svg() {
                                        svg.size(px(icon_back_size))
                                            .text_color(theme.secondary_text)
                                            .into_any_element()
                                    } else {
                                        img(icon).size(px(icon_back_size)).into_any_element()
                                    })
                                },
                            )
                        }),
                )
            })
            .child(div().w_auto().child(self.text_input.clone()))
            .children(self.variable_input.iter().cloned().map(|ipt| {
                div().child(deferred(ipt)).with_animation(
                    "variable-input",
                    Animation::new(Duration::from_millis(100)).with_easing(Ease::ease_in_out_sine),
                    |this, frac| this.opacity(frac),
                )
            }))
            .border_b_2()
            .border_color(theme.border)
    }

    fn render_config_banner(&self, theme: Arc<ThemeData>) -> impl IntoElement {
        div()
            .w_full()
            .px_4()
            .py(px(6.))
            .bg(theme.banner_bg)
            .border_b_1()
            .border_color(theme.banner_border)
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.banner_text)
                    .child("⚠  Running with default config — run "),
            )
            .child(
                div()
                    .px(px(4.))
                    .py(px(1.))
                    .rounded_sm()
                    .bg(theme.bg_code)
                    .text_size(px(11.0))
                    .text_color(theme.text_code)
                    .font_family("monospace")
                    .child("sherlock init"),
            )
    }

    fn render_mode_label(&self, theme: Arc<ThemeData>) -> impl IntoElement {
        div()
            .px(px(14.))
            .py(px(4.))
            .text_size(px(14.))
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_mode_label)
            .font_family(theme.font_family.clone())
            .child(self.navigation.current().mode.display_str())
    }

    fn render_results(&self, cx: &mut Context<Self>, _theme: Arc<ThemeData>) -> impl IntoElement {
        let (indices, data) = self
            .navigation
            .with_model(cx, |mdl| (mdl.filtered_indices(), mdl.data()));
        let sidebar = self
            .navigation
            .with_selected_item(cx, |selected_item, cx| selected_item.sidebar(cx))
            .and_then(|s| s);
        let EntityStyle::Row {
            state,
            selected_index,
        } = &self.navigation.current().style
        else {
            return div().id("results-container");
        };
        let query = self.text_input.read(cx).content.clone();

        let theme = cx.global::<ActiveTheme>().0.clone();
        let max_shortcuts = ConfigGuard::read().map_or(5, |c| c.appearance.num_shortcuts) as usize;
        div()
            .id("results-container")
            .size_full()
            .px(px(10.))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .flex_1()
                    .min_h_0()
                    .size_full()
                    .child({
                        let selected_idx = *selected_index;
                        let view_handle = cx.entity().clone();
                        list(state.clone(), {
                            let theme = theme.clone();
                            move |idx, _win, cx| {
                                let data_idx = match indices.get(idx) {
                                    Some(&i) => i,
                                    None => return div().into_any_element(),
                                };
                                let data_snapshot = data.read(cx).clone();
                                let child = match data_snapshot.get(data_idx) {
                                    Some(c) => c,
                                    None => return div().into_any_element(),
                                };

                                let shortcut_idx = child
                                    .shortcut()
                                    .then(|| {
                                        let count = indices[..idx.min(indices.len())]
                                            .iter()
                                            .filter(|&&i| {
                                                data_snapshot.get(i).is_some_and(|c| c.shortcut())
                                            })
                                            .take(max_shortcuts)
                                            .count();
                                        if count > max_shortcuts - 1 {
                                            None
                                        } else {
                                            Some(count + 1)
                                        }
                                    })
                                    .flatten();

                                Self::render_list_item(
                                    child,
                                    shortcut_idx,
                                    Selection::new(data_idx, idx == selected_idx),
                                    &query,
                                    theme.clone(),
                                    view_handle.clone(),
                                    cx,
                                )
                            }
                        })
                        .pb(px(5.))
                        .size_full()
                    })
                    .when_some(sidebar, |this, sidebar| {
                        this.child(
                            div()
                                .h_full()
                                .w_full()
                                .overflow_x_hidden()
                                .pb(px(10.))
                                .child(
                                    div()
                                        .id("sidebar")
                                        .overflow_y_scroll()
                                        .overflow_x_hidden()
                                        .size_full()
                                        .p(px(16.))
                                        .rounded_lg()
                                        .bg(theme.bg_selected)
                                        .border_1()
                                        .border_color(theme.border_selected)
                                        .flex_col()
                                        .child(sidebar)
                                        .with_animation(
                                            "sidebar-pop-in",
                                            Animation::new(Duration::from_millis(200))
                                                .with_easing(Ease::ease_out_expo),
                                            |this, frac| this.opacity(frac),
                                        ),
                                )
                                .w(px(400.)),
                        )
                    }),
            )
            .child(self.render_context_menu(theme))
    }

    fn render_result_grid(
        &self,
        cx: &mut Context<Self>,
        theme: Arc<ThemeData>,
    ) -> impl IntoElement {
        let (indices, data) = self
            .navigation
            .with_model(cx, |mdl| (mdl.filtered_indices(), mdl.data()));

        let style = &self.navigation.current().style;
        let EntityStyle::Grid {
            scroll_handle,
            selected_index,
            columns,
            ..
        } = style
        else {
            return div().id("results-container");
        };

        let selected_idx = *selected_index;
        let col_count = *columns;

        let query = self.text_input.read(cx).content.clone();

        div()
            .id("results-container")
            .relative()
            .size_full()
            .px(px(10.))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .flex_1()
                    .min_h_0()
                    .size_full()
                    .child(
                        gpui::uniform_list("emoji-grid", indices.len().div_ceil(col_count), {
                            let theme = theme.clone();
                            let handle = cx.entity().clone();
                            move |range, _win, cx| {
                                range
                                    .map(|row_idx| {
                                        div()
                                            .flex()
                                            .flex_row()
                                            .w_full()
                                            .gap(px(2.0))
                                            .children((0..col_count).map(|col_idx| {
                                                let item_idx = row_idx * col_count + col_idx;

                                                if let Some(&data_idx) = indices.get(item_idx) {
                                                    let data_snapshot = data.read(cx).clone();
                                                    if let Some(child) = data_snapshot.get(data_idx)
                                                    {
                                                        return div()
                                                            .w_0()
                                                            .flex_1()
                                                            .child(Self::render_list_item(
                                                                child,
                                                                None,
                                                                Selection::new(
                                                                    data_idx,
                                                                    item_idx == selected_idx,
                                                                ),
                                                                &query,
                                                                theme.clone(),
                                                                handle.clone(),
                                                                cx,
                                                            ))
                                                            .into_any_element();
                                                    }
                                                }
                                                // Empty cell for alignment
                                                div().flex_1().into_any_element()
                                            }))
                                            .into_any_element()
                                    })
                                    .collect::<Vec<_>>()
                            }
                        })
                        .gap(px(2.0))
                        .track_scroll(scroll_handle)
                        .size_full(),
                    ),
            )
            .child(self.render_context_menu(theme))
    }

    fn render_context_menu(&self, theme: Arc<ThemeData>) -> impl IntoElement {
        if let Some(active) = self.context_idx {
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
                    .flex()
                    .flex_col()
                    .min_w(px(200.))
                    .gap(px(5.))
                    .children(self.context_actions.iter().enumerate().map(|(i, child)| {
                        let is_selected = i == active;
                        match child.as_ref() {
                            ContextMenuAction::App(_) => child
                                .render_row(is_selected, theme.clone())
                                .into_any_element(),
                            ContextMenuAction::Fn(_) => child
                                .render_row(is_selected, theme.clone())
                                .into_any_element(),
                            ContextMenuAction::Emoji(_) => child
                                .render_emoji_col(is_selected, theme.clone())
                                .into_any_element(),
                        }
                    }))
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

    fn render_status_bar(&self, theme: Arc<ThemeData>, cx: &mut Context<Self>) -> impl IntoElement {
        let message_count = self.navigation.message_count(cx);
        div()
            .h(px(30.))
            .line_height(px(30.))
            .w_full()
            .flex()
            .bg(theme.bg_status_bar)
            .border_t_1()
            .border_color(theme.border)
            .px_5()
            .text_size(px(13.))
            .font_family(theme.font_family.clone())
            .items_center()
            .text_color(theme.text_status_bar)
            .child("Sherlock")
            .when(message_count > 0, |this| {
                this.child(self.render_error_indicator(message_count, theme.clone(), cx))
            })
            .child(div().flex_1())
            .child(self.render_context_hint(cx, theme.clone()))
    }

    fn render_error_indicator(
        &self,
        count: usize,
        theme: Arc<ThemeData>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .ml(px(10.))
            .h_full()
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(5.))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.text_input.update(cx, |this, _| this.reset());
                    this.navigation.set_messages_active();
                    this.filter_and_sort(cx);
                }),
            )
            .when(count > 0, |this| {
                this.child(
                    div()
                        .h_full()
                        .px(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded_sm()
                        .border_1()
                        .border_color(theme.color_err)
                        .text_color(theme.color_err)
                        .bg(theme.color_err.alpha(0.4))
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .child(count.to_string()),
                )
            })
    }

    fn render_context_hint(
        &self,
        _cx: &mut Context<Self>,
        theme: Arc<ThemeData>,
    ) -> impl IntoElement {
        if self.has_actions {
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().mr_1().child(SharedString::from("Additional Actions")))
                .children(
                    get_context_key_parts()
                        .into_iter()
                        .map(|p| keybind_box(p, &theme)),
                )
        } else {
            div()
        }
    }

    pub fn render_list_item(
        ad: &RenderableChild,
        shortcut_idx: Option<usize>,
        selection: Selection,
        query: &str,
        theme: Arc<ThemeData>,
        handle: Entity<LauncherView>,
        cx: &mut App,
    ) -> AnyElement {
        let idx = selection.data_idx;
        div()
            .relative()
            .id(selection.data_idx)
            .w_full()
            .on_click(move |evt: &ClickEvent, win, cx: &mut App| {
                if !evt.standard_click() && !evt.is_keyboard() {
                    return;
                }

                let n_clicks = ConfigGuard::read()
                    .ok()
                    .and_then(|c| c.behavior.n_clicks)
                    .unwrap_or(2);

                if evt.click_count() as u8 >= n_clicks {
                    handle.update(cx, move |this, cx| {
                        this.navigation.set_selected_data_idx(idx, cx);
                        this.execute_listener(&Execute {}, win, cx);
                    })
                } else {
                    handle.update(cx, move |this, cx| {
                        this.navigation.set_selected_data_idx(idx, cx);
                    })
                }
            })
            .child(
                div()
                    .group("")
                    .relative()
                    .mb(px(5.0))
                    .w_full()
                    .cursor_pointer()
                    .when(!ad.handles_borders(), |this| {
                        this.list_item_border(&theme, &selection)
                    })
                    .child(ad.render(selection, query, theme.clone(), cx))
                    .when_some(shortcut_idx, |this, shortcut_idx| {
                        this.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_end()
                                .pr_8()
                                .child(
                                    div()
                                        .px(px(6.5))
                                        .flex()
                                        .justify_center()
                                        .items_center()
                                        .gap(px(2.))
                                        .h(px(28.))
                                        .border_1()
                                        .rounded_sm()
                                        .bg(theme.bg_code)
                                        .border_color(theme.border_selected)
                                        .text_size(px(12.))
                                        .text_color(theme.secondary_text)
                                        .when(selection.is_selected, |this| {
                                            this.aspect_square()
                                                .child(div().child("↵").relative().top(px(1.)))
                                        })
                                        .when(!selection.is_selected, |this| {
                                            this.children(
                                                ShortcutKeyMod::get()
                                                    .map(|mods| {
                                                        mods.iter().map(|c| {
                                                            div()
                                                                .flex()
                                                                .justify_center()
                                                                .min_w(px(10.))
                                                                .child(c.to_string())
                                                        })
                                                    })
                                                    .into_iter()
                                                    .flatten(),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .justify_center()
                                                    .min_w(px(10.))
                                                    .child(shortcut_idx.to_string()),
                                            )
                                        }),
                                ),
                        )
                    }),
            )
            .with_animation(
                "fade-in-results",
                Animation::new(Duration::from_millis(500)).with_easing(Ease::ease_out_expo),
                |this, frac| this.opacity(frac),
            )
            .into_any_element()
    }
}

fn get_context_key_parts() -> Vec<String> {
    CONTEXT_MENU_BIND
        .get_or_init(|| {
            ConfigGuard::read()
                .ok()
                .and_then(|config| {
                    config
                        .keybinds
                        .0
                        .iter()
                        .find(|(_, func)| **func == UIFunction::ToggleContext)
                        .map(|(name, _)| name.clone())
                })
                .unwrap_or_else(|| "ctrl-l".to_string())
        })
        .split('-')
        .map(|part| match part {
            "ctrl" => "⌃".to_string(),
            "cmd" => "⌘".to_string(),
            "shift" => "⇧".to_string(),
            "alt" => "⌥".to_string(),
            other if other.len() == 1 => other.to_uppercase(),
            other => other.to_string(),
        })
        .collect()
}

fn keybind_box(text: String, theme: &Arc<ThemeData>) -> impl Element {
    div()
        .flex_none()
        .p(px(5.))
        .bg(theme.bg_keybind)
        .rounded_sm()
        .text_size(px(11.))
        .line_height(relative(1.0))
        .child(text)
}
