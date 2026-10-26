use gpui::{
    AnyElement, App, ClickEvent, Context, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px, rgb,
};

use crate::{
    app::theme::ThemeData,
    ui::{
        components::{
            color_swatch::color_swatch, keybind::keybind, select::select, stepper::stepper,
            toggle::toggle,
        },
        launcher::navigator::views::MoveDirection,
        model::data::NoData,
        views::{Chrome, LauncherEvent, Propagation, ShellCtx, SherlockView},
    },
};

/// The value of one setting; decides which control is shown.
#[derive(Clone, Debug)]
pub enum SettingValue {
    Toggle(bool),
    Color(Hsla),
    Keybind(Vec<SharedString>),
    Choice {
        options: &'static [&'static str],
        selected: usize,
    },
    Number {
        value: i64,
        min: i64,
        max: i64,
        step: i64,
    },
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub label: &'static str,
    pub description: &'static str,
    pub value: SettingValue,
}

#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub title: &'static str,
    pub settings: Vec<Setting>,
}

/// What a click on a control does to its value.
#[derive(Clone, Copy, Debug)]
enum Edit {
    Toggle,
    NextChoice,
    Step(i64),
}

impl SettingValue {
    fn apply(&mut self, edit: Edit) {
        match (self, edit) {
            (Self::Toggle(on), Edit::Toggle) => *on = !*on,
            (Self::Choice { options, selected }, Edit::NextChoice) => {
                *selected = (*selected + 1) % options.len().max(1);
            }
            (
                Self::Number {
                    value, min, max, ..
                },
                Edit::Step(delta),
            ) => *value = (*value + delta).clamp(*min, *max),
            _ => {}
        }
    }
}

fn setting(label: &'static str, description: &'static str, value: SettingValue) -> Setting {
    Setting {
        label,
        description,
        value,
    }
}

fn keys(keys: &[&'static str]) -> SettingValue {
    SettingValue::Keybind(keys.iter().map(|&k| SharedString::from(k)).collect())
}

/// Placeholder data until settings are read from the config.
pub fn dummy_sections() -> Vec<SettingsSection> {
    use SettingValue::*;
    vec![
        SettingsSection {
            title: "General",
            settings: vec![
                setting(
                    "Launch on startup",
                    "Start the Sherlock daemon when you log in.",
                    Toggle(true),
                ),
                setting(
                    "Close after run",
                    "Hide the launcher after an item was executed.",
                    Toggle(true),
                ),
                setting(
                    "Clicks to run",
                    "How many clicks run an item; fewer just select it.",
                    Number {
                        value: 2,
                        min: 1,
                        max: 3,
                        step: 1,
                    },
                ),
            ],
        },
        SettingsSection {
            title: "Appearance",
            settings: vec![
                setting(
                    "Theme",
                    "Color scheme of the launcher.",
                    Choice {
                        options: &["System", "Dark", "Light"],
                        selected: 0,
                    },
                ),
                setting(
                    "Accent color",
                    "Used for selections and highlights.",
                    Color(rgb(0x89b4fa).into()),
                ),
                setting(
                    "Background",
                    "Window background.",
                    Color(rgb(0x1e1e2e).into()),
                ),
                setting(
                    "Shortcut count",
                    "How many results get a numbered shortcut.",
                    Number {
                        value: 5,
                        min: 0,
                        max: 9,
                        step: 1,
                    },
                ),
            ],
        },
        SettingsSection {
            title: "Keybinds",
            settings: vec![
                setting(
                    "Next item",
                    "Move the selection down.",
                    keys(&["ctrl", "n"]),
                ),
                setting(
                    "Previous item",
                    "Move the selection up.",
                    keys(&["ctrl", "p"]),
                ),
                setting(
                    "Context menu",
                    "Open actions for the selected item.",
                    keys(&["ctrl", "l"]),
                ),
                setting("Quit", "Close the launcher.", keys(&["escape"])),
            ],
        },
        SettingsSection {
            title: "Search",
            settings: vec![
                setting(
                    "Learn from picks",
                    "Rank items you pick often higher for the same query.",
                    Toggle(true),
                ),
                setting(
                    "Fuzzy matching",
                    "Also match slightly misspelled queries.",
                    Toggle(false),
                ),
            ],
        },
    ]
}

pub struct SettingsView {
    sections: Vec<SettingsSection>,
    /// Index of the section shown on the right.
    active: usize,
    /// Held while the page is shown; keys go to the page's root first, then bubble to the shell.
    focus: FocusHandle,
}

impl SettingsView {
    pub fn new(cx: &mut App) -> Self {
        Self {
            sections: dummy_sections(),
            active: 0,
            focus: cx.focus_handle(),
        }
    }

    fn edit(&mut self, section: usize, row: usize, edit: Edit) {
        apply_edit(&mut self.sections, section, row, edit);
    }

    /// Click handler that applies `edit` to setting `row` of `section`.
    fn on_edit(
        cx: &mut Context<Self>,
        section: usize,
        row: usize,
        edit: Edit,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.edit(section, row, edit);
            cx.notify();
        })
    }

    fn render_nav(&self, theme: &ThemeData, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("settings-nav")
            .flex_none()
            .w(px(180.))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(px(8.))
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.bg_status_bar)
            .child(
                div()
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(10.))
                    .text_size(px(15.))
                    .text_color(theme.primary_text)
                    .child("Settings"),
            )
            .children(self.sections.iter().enumerate().map(|(i, section)| {
                let active = i == self.active;
                div()
                    .id(("settings-section", i))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded_md()
                    .cursor_pointer()
                    .text_size(px(13.))
                    .map(|this| {
                        if active {
                            this.bg(theme.bg_selected).text_color(theme.primary_text)
                        } else {
                            this.text_color(theme.secondary_text)
                                .hover(|s| s.bg(theme.bg_idle))
                        }
                    })
                    .child(section.title)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.active = i;
                        cx.notify();
                    }))
            }))
    }

    fn render_control(
        value: &SettingValue,
        section: usize,
        row: usize,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = ("setting", section * 1000 + row);
        match value {
            SettingValue::Toggle(on) => toggle(
                id,
                *on,
                theme,
                Self::on_edit(cx, section, row, Edit::Toggle),
            )
            .into_any_element(),
            SettingValue::Color(color) => {
                // TODO: open a color picker
                color_swatch(id, *color, theme, |_, _, _| {}).into_any_element()
            }
            SettingValue::Keybind(keys) => {
                // TODO: start recording a new combination
                keybind(id, keys, theme, |_, _, _| {}).into_any_element()
            }
            SettingValue::Choice { options, selected } => select(
                id,
                options.get(*selected).copied().unwrap_or_default(),
                theme,
                Self::on_edit(cx, section, row, Edit::NextChoice),
            )
            .into_any_element(),
            SettingValue::Number {
                value,
                min,
                max,
                step,
            } => stepper(
                id,
                *value,
                *min,
                *max,
                theme,
                Self::on_edit(cx, section, row, Edit::Step(-step)),
                Self::on_edit(cx, section, row, Edit::Step(*step)),
            )
            .into_any_element(),
        }
    }

    fn render_section(&self, theme: &ThemeData, cx: &mut Context<Self>) -> impl IntoElement {
        let section_idx = self.active;
        let section = self.sections.get(section_idx);

        div()
            .id("settings-content")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .p(px(20.))
            .flex()
            .flex_col()
            .when_some(section, |this, section| {
                this.child(
                    div()
                        .pb(px(12.))
                        .text_size(px(18.))
                        .text_color(theme.primary_text)
                        .child(section.title),
                )
                .children(section.settings.iter().enumerate().map(|(row, setting)| {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .py(px(12.))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.primary_text)
                                        .child(setting.label),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.secondary_text)
                                        .child(setting.description),
                                ),
                        )
                        .child(Self::render_control(
                            &setting.value,
                            section_idx,
                            row,
                            theme,
                            cx,
                        ))
                }))
            })
    }
}

/// Applies `edit` to setting `row` of `section`, if it exists.
fn apply_edit(sections: &mut [SettingsSection], section: usize, row: usize, edit: Edit) {
    if let Some(setting) = sections
        .get_mut(section)
        .and_then(|s| s.settings.get_mut(row))
    {
        setting.value.apply(edit);
    }
}

impl SherlockView for SettingsView {
    type Data = NoData;

    fn chrome(&self) -> Chrome {
        Chrome::NONE
    }

    fn focus_target(&self, _cx: &App) -> Option<FocusHandle> {
        Some(self.focus.clone())
    }

    /// Up/Down switch sections; everything else (back, quit, ...) keeps the default.
    fn on_event(
        &mut self,
        event: &LauncherEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        let last = self.sections.len().saturating_sub(1);
        self.active = match event {
            LauncherEvent::Move(MoveDirection::Up) => self.active.saturating_sub(1),
            LauncherEvent::Move(MoveDirection::Down) => (self.active + 1).min(last),
            _ => return Propagation::Continue,
        };
        cx.notify();
        Propagation::Stop
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = shell.theme.clone();
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .bg(theme.bg_app)
            .font_family(theme.font_family.clone())
            .child(self.render_nav(&theme, cx))
            .child(self.render_section(&theme, cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips() {
        let mut v = SettingValue::Toggle(false);
        v.apply(Edit::Toggle);
        assert!(matches!(v, SettingValue::Toggle(true)));
    }

    #[test]
    fn choice_wraps() {
        let mut v = SettingValue::Choice {
            options: &["a", "b"],
            selected: 1,
        };
        v.apply(Edit::NextChoice);
        assert!(matches!(v, SettingValue::Choice { selected: 0, .. }));
    }

    #[test]
    fn number_is_clamped() {
        let mut v = SettingValue::Number {
            value: 3,
            min: 1,
            max: 3,
            step: 1,
        };
        v.apply(Edit::Step(1));
        assert!(matches!(v, SettingValue::Number { value: 3, .. }));
        v.apply(Edit::Step(-5));
        assert!(matches!(v, SettingValue::Number { value: 1, .. }));
    }

    #[test]
    fn mismatched_edit_is_ignored() {
        let mut v = SettingValue::Toggle(true);
        v.apply(Edit::Step(1));
        assert!(matches!(v, SettingValue::Toggle(true)));
    }

    #[test]
    fn edit_targets_one_row() {
        let mut sections = dummy_sections();
        let before = sections[0].settings[1].value.clone();
        apply_edit(&mut sections, 0, 0, Edit::Toggle);
        assert!(matches!(
            sections[0].settings[0].value,
            SettingValue::Toggle(false)
        ));
        assert!(matches!(
            (&before, &sections[0].settings[1].value),
            (SettingValue::Toggle(a), SettingValue::Toggle(b)) if a == b
        ));
    }
}
