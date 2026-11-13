use std::{sync::Arc, time::Duration};

use gpui::{
    Animation,
    AnimationExt,
    AnyElement,
    App,
    ClickEvent,
    Context,
    Div,
    ElementId,
    Focusable,
    FontWeight,
    InteractiveElement,
    IntoElement,
    Keystroke,
    MouseDownEvent,
    ParentElement,
    Render,
    SharedString,
    StatefulInteractiveElement,
    Styled,
    Window,
    deferred,
    div,
    prelude::FluentBuilder,
    px,
};

use crate::{
    CONTEXT_MENU_BIND,
    app::{
        bindings::ShortcutKeyMod,
        theme::{ActiveTheme, ThemeData},
    },
    config::ConfigGuard,
    launcher::{LauncherValues, utils::exec_mode::ExecMode},
    loader::resolve_icon_path,
    ui::{
        actions::UIFunction,
        components::status_bar::{self, KeySpec, StatusItem, StatusKind},
        launcher::{Execute, LauncherMode, LauncherView, execute::ExecRequest},
        model::ItemIdx,
        traits::RenderableChildDelegate,
        utils::{ease::Ease, render::ListItemBorder, selection::Selection},
        views::{Background, Chrome, ShellCtx, list::RowCtx},
        widgets::RenderableChild,
    },
};

impl Render for LauncherView {
    fn render(&mut self, win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();

        let selected = self.navigation.selected_key(cx);
        if selected != self.last_selected {
            if let Some(old) = self.last_selected.take() {
                self.navigation.with_item_by_key(old, cx, |item, cx| {
                    if let Some(item) = item.as_launcher_item() {
                        item.reset_inner(cx);
                    }
                });
            }
            self.last_selected = selected;
        }

        self.check_actions(cx);
        self.sidebar_controller.check_validity(&self.navigation, cx);

        let chrome = self.navigation.current().handle.chrome(cx);
        // home mode with a query is a search
        let searching = *self.navigation.current_mode() == LauncherMode::Home
            && !self.text_input.read(cx).content.is_empty();
        self.ensure_focus(chrome, win, cx);
        let sidebar = self.render_sidebar(chrome, &theme, cx);

        div()
            .id("sherlock")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg_app)
            .border_2()
            .border_color(theme.border)
            .rounded_xl()
            .overflow_hidden()
            .on_action(cx.listener(Self::selection_up))
            .on_action(cx.listener(Self::selection_down))
            .on_action(cx.listener(Self::selection_left))
            .on_action(cx.listener(Self::selection_right))
            .on_action(cx.listener(Self::next_var))
            .on_action(cx.listener(Self::prev_var))
            .on_action(cx.listener(Self::maximize_listener))
            .on_action(cx.listener(Self::minimize_listener))
            .on_action(cx.listener(Self::execute_listener))
            .on_action(cx.listener(Self::execute_inplace_listener))
            .on_action(cx.listener(Self::shortcut_listener))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::open_context))
            .on_action(cx.listener(Self::error_page))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::copy_listener))
            .on_key_down(cx.listener(Self::context_menu_binds_listener))
            .when(chrome.search_bar, |this| {
                this.child(self.render_search_bar(theme.clone(), searching))
            })
            // main area: content column | sidebar (full height between search and status bar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .relative() // the context menu anchors here
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .when(!self.config_initialized, |this| {
                                this.child(self.render_config_banner(theme.clone()))
                            })
                            .when(chrome.mode_label, |this| {
                                this.child(self.render_mode_label(theme.clone(), searching))
                            })
                            .child(div().flex_1().min_h_0().child(self.render_body(
                                chrome,
                                theme.clone(),
                                win,
                                cx,
                            ))),
                    )
                    .when_some(sidebar, |this, sidebar| this.child(sidebar))
                    .child(self.context_menu.render(theme.clone())),
            )
            .when(chrome.status_bar, |this| {
                this.child(self.render_status_bar(theme, win, cx))
            })
    }
}

impl LauncherView {
    /// Keeps keyboard focus inside the launcher.
    ///
    /// Preferred target: the view's `focus_target`, else the search input if it's shown, else
    /// the launcher root. Focus is moved only when it's invalid: on a hidden search input, on
    /// nothing, outside the launcher, or parked on the root while something better exists.
    fn ensure_focus(&mut self, chrome: Chrome, win: &mut Window, cx: &mut Context<Self>) {
        let input = self.text_input.focus_handle(cx);
        let preferred = self
            .navigation
            .current()
            .handle
            .focus_target(cx)
            .or_else(|| chrome.search_bar.then(|| input.clone()))
            .unwrap_or_else(|| self.focus_handle.clone());

        let in_search_bar = input.is_focused(win)
            || self
                .variable_input
                .iter()
                .any(|v| v.focus_handle(cx).is_focused(win));

        let valid = if in_search_bar {
            // the search bar only counts while it's rendered
            chrome.search_bar
        } else if self.focus_handle.is_focused(win) {
            // the root is a fallback, not a destination
            preferred.is_focused(win)
        } else {
            // something inside the view (as of the last frame)
            self.focus_handle.contains_focused(win, cx)
        };

        if !valid {
            preferred.focus(win, cx);
        }
    }

    fn render_search_bar(&self, theme: Arc<ThemeData>, searching: bool) -> impl IntoElement {
        let search_icon = ConfigGuard::read_with(|c| c.search_bar_icon.clone()).unwrap_or_default();

        // Extract everything upfront — avoid repeated unwraps inside closures
        let icon_enabled = search_icon.enable;
        let icon_path = search_icon.icon.clone();
        let icon_size = search_icon.icon_size as f32;
        let icon_back_path = search_icon.icon_back.clone();
        let icon_back_size = search_icon.icon_back_size as f32;

        div()
            .flex()
            .flex_row()
            .w_full()
            .items_center()
            .px_4()
            .py(px(6.))
            .gap_3()
            .when(icon_enabled, |this| {
                this.child(
                    div()
                        .flex()
                        .size(px(24.))
                        .items_center()
                        .justify_center()
                        .when(!searching, |this| {
                            this.when_some(resolve_icon_path(&icon_path), |this, icon| {
                                this.child(icon.element(px(icon_size), theme.secondary_text))
                            })
                        })
                        .when(searching, |this| {
                            this.when_some(resolve_icon_path(&icon_back_path), |this, icon| {
                                this.child(icon.element(px(icon_back_size), theme.secondary_text))
                            })
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
            .border_b_1()
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

    fn render_mode_label(&self, theme: Arc<ThemeData>, searching: bool) -> impl IntoElement {
        div()
            .px(px(14.))
            .py(px(4.))
            .text_size(px(14.))
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_mode_label)
            .font_family(theme.typography.font_family.clone())
            .child(self.navigation.current().mode.display_str(searching))
    }

    /// The selected item's sidebar, if the view shows one and the item has one. Rendered next to
    /// the content column, so it spans the full height between search bar and status bar.
    fn render_sidebar(
        &mut self,
        chrome: Chrome,
        theme: &Arc<ThemeData>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !chrome.sidebar {
            return None;
        }
        let content = self
            .navigation
            .with_selected_item(cx, |item, cx| {
                let allowed = item.sidebar_default_on() || self.sidebar_controller.is_valid();
                allowed.then(|| item.sidebar(cx)).flatten()
            })
            .flatten()?;
        self.sidebar_controller.sync_scroll();

        Some(
            div()
                .id("sidebar")
                .flex_none()
                .w(px(300.))
                .h_full()
                .overflow_y_scroll()
                .overflow_x_hidden()
                .track_scroll(self.sidebar_controller.scroll_handle())
                .p(px(16.))
                .border_l_1()
                .border_color(theme.border)
                .child(content)
                .with_animation(
                    "sidebar-pop-in",
                    Animation::new(Duration::from_millis(200)).with_easing(Ease::ease_out_expo),
                    |this, frac| this.opacity(frac),
                )
                .into_any_element(),
        )
    }

    /// The active view's body and the context menu.
    fn render_body(
        &mut self,
        chrome: Chrome,
        theme: Arc<ThemeData>,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let nav = self.navigation.current();
        let is_grid = nav.layout.grid.is_some();
        let shell = ShellCtx {
            shell: cx.entity(),
            query: self.text_input.read(cx).content.clone(),
            theme: theme.clone(),
            layout: &nav.layout,
        };
        let body = nav.handle.render_body(&shell, win, cx);

        div()
            .id("results-container")
            .when(is_grid, |this| this.relative())
            .size_full()
            .when(chrome.inset, |this| this.px(px(10.)))
            .child(body)
    }

    /// The active view's status bar (`SherlockView::status_bar`), shell items filled in.
    fn render_status_bar(
        &self,
        theme: Arc<ThemeData>,
        win: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bar = self.navigation.current().handle.status_bar(cx);
        let n = bar.left.len();

        div()
            .h(px(status_bar::HEIGHT))
            .line_height(px(status_bar::HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .gap(px(status_bar::GAP))
            .bg(theme.bg_status_bar)
            .rounded_b(px(10.))
            .border_t_1()
            .border_color(theme.border)
            .px_5()
            .text_size(px(13.))
            .font_family(theme.typography.font_family.clone())
            .text_color(theme.text_status_bar)
            .children(
                (bar.left.iter().enumerate())
                    .filter_map(|(i, item)| self.render_status_item(item, i, &theme, win, cx)),
            )
            .child(div().flex_1())
            .children(
                (bar.right.iter().enumerate())
                    .filter_map(|(i, item)| self.render_status_item(item, n + i, &theme, win, cx)),
            )
    }

    /// One item; `None` when a shell item has nothing to show.
    fn render_status_item(
        &self,
        item: &StatusItem,
        id: usize,
        theme: &Arc<ThemeData>,
        win: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let el = match &item.kind {
            StatusKind::Keys { keys, label } => {
                let (caps, press) = self.resolve_keys(*keys, win, cx)?;
                let el = status_bar::keys(caps, label.clone(), theme);
                // A bind's hint without its own handler presses the bind's key.
                if let (None, Some(stroke)) = (item.on_click, press) {
                    return Some(el.cursor_pointer().on_mouse_down(
                        gpui::MouseButton::Left,
                        move |_, win, cx| {
                            win.dispatch_keystroke(stroke.clone(), cx);
                        },
                    ));
                }
                el
            }
            StatusKind::Brand => self.render_brand(cx),
            StatusKind::Messages => match self.navigation.message_count(cx) {
                0 => return None,
                count => self.render_error_indicator(count, theme, cx),
            },
            StatusKind::Minimized => {
                let status = self.navigation.minimized_status(cx)?;
                self.render_minimized_indicator(status, theme, cx)
            }
            StatusKind::ContextActions => {
                if !self.context_menu.has_actions {
                    return None;
                }
                status_bar::keys(
                    context_key_parts().iter().cloned(),
                    SharedString::new_static("Additional Actions"),
                    theme,
                )
            }
            kind => status_bar::view_item(kind, id, theme)?,
        };
        Some(match item.on_click {
            Some(f) => el
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, move |_, win, cx| f(win, cx)),
            None => el,
        })
    }

    /// A hint's caps, looked up in the keymap or the view's binds; `None` when unbound. A bind
    /// also returns its keystroke, so clicking the hint can press it.
    fn resolve_keys(
        &self,
        spec: KeySpec,
        win: &Window,
        cx: &App,
    ) -> Option<(Vec<SharedString>, Option<Keystroke>)> {
        match spec {
            KeySpec::Fixed(keys) => Some((
                keys.iter().map(|k| SharedString::new_static(k)).collect(),
                None,
            )),
            KeySpec::Action(action) => {
                let binding = win.highest_precedence_binding_for_action(&*action())?;
                let stroke = binding.keystrokes().first()?;
                Some((status_bar::key_caps(stroke.modifiers(), stroke.key()), None))
            }
            KeySpec::Bind(name) => {
                let stroke = self.navigation.current().handle.bind_key(name, cx)?;
                Some((
                    status_bar::key_caps(&stroke.modifiers, &stroke.key),
                    Some(stroke),
                ))
            }
        }
    }

    /// "Sherlock"; click shows the credits.
    fn render_brand(&self, cx: &mut Context<Self>) -> Div {
        div()
            .child("Sherlock")
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, win, cx| {
                    this.run(
                        ExecRequest {
                            mode: ExecMode::ShowCredits,
                            ..Default::default()
                        },
                        win,
                        cx,
                    );
                }),
            )
            .cursor_pointer()
    }

    /// A minimized view (e.g. an AI answer still streaming): its status, click to bring it back.
    fn render_minimized_indicator(
        &self,
        status: Background,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .h(px(20.))
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(8.))
            .rounded(px(10.))
            .bg(theme.bg_badge)
            .cursor_pointer()
            .child(status_bar::status_dot(&status.busy, "minimized-dot", theme))
            .child(status.label)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    if this.navigation.restore_minimized() {
                        this.text_input.update(cx, |input, _| input.reset());
                        this.filter_and_sort(cx);
                        cx.notify();
                    }
                }),
            )
    }

    fn render_error_indicator(
        &self,
        count: usize,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
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
                    this.navigation.show_messages();
                    this.filter_and_sort(cx);
                }),
            )
            .child(
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
                    .font_family(theme.typography.font_family.clone())
                    .child(count.to_string()),
            )
    }

    /// A launcher item's row: its own content inside the shared row frame.
    pub fn render_list_item(
        ad: &RenderableChild,
        row: &RowCtx<ItemIdx>,
        cx: &mut App,
    ) -> AnyElement {
        let selection = Selection::new(row.id, row.selected);
        let content = ad.render(selection, row.query, row.theme.clone(), cx);
        Self::render_row_frame(row, ad.handles_borders(), content)
    }

    /// The shared row frame around any item's `content`: click to select / run, selection
    /// border (unless `borderless`), shortcut badge and fade-in.
    pub fn render_row_frame<Id>(
        row: &RowCtx<Id>,
        borderless: bool,
        content: AnyElement,
    ) -> AnyElement {
        let shortcut_idx = row.shortcut;
        let selected = row.selected;
        let key = row.key;
        let theme = row.theme.clone();
        let handle = row.shell.clone();
        div()
            .relative()
            .id(ElementId::Integer(key.as_u64()))
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
                        this.navigation.select_key(key, cx);
                        if !this.context_menu.is_open() {
                            this.execute_listener(&Execute {}, win, cx);
                        }
                    })
                } else {
                    handle.update(cx, move |this, cx| {
                        this.navigation.select_key(key, cx);
                        if this.context_menu.is_open() {
                            this.context_menu.close();
                        }
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
                    .when(!borderless, |this| {
                        this.list_item_border_for(&theme, selected)
                    })
                    .child(content)
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
                                        .when(selected, |this| {
                                            this.aspect_square()
                                                .child(div().child("↵").relative().top(px(1.)))
                                        })
                                        .when(!selected, |this| {
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

/// The context menu's key as caps (`⌃`, `L`), read from the config once.
fn context_key_parts() -> &'static [SharedString] {
    static PARTS: std::sync::OnceLock<Vec<SharedString>> = std::sync::OnceLock::new();
    PARTS.get_or_init(|| {
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
            .map(|part| {
                SharedString::from(match part {
                    "ctrl" => "⌃".to_string(),
                    "cmd" => "⌘".to_string(),
                    "shift" => "⇧".to_string(),
                    "alt" => "⌥".to_string(),
                    other if other.len() == 1 => other.to_uppercase(),
                    other => other.to_string(),
                })
            })
            .collect()
    })
}
