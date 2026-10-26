use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div,
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
        views::{Chrome, LauncherEvent, Propagation, Query, QueryResult, ShellCtx, SherlockView},
    },
};

/// The value of one setting; decides which control is shown.
#[derive(Clone, Debug, PartialEq)]
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
    /// Sub-heading within the section (`// startup`); consecutive settings share one.
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub value: SettingValue,
}

impl Setting {
    /// `query` must be lowercase.
    fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query) || self.description.to_lowercase().contains(query)
    }
}

#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub title: &'static str,
    /// One line under the section's title.
    pub description: &'static str,
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

    /// What Enter does: flip, next option, or step up (wrapping to `min` at `max`).
    /// `None` for values that need their own editor (color, keybind).
    fn activate(&self) -> Option<Edit> {
        match *self {
            Self::Toggle(_) => Some(Edit::Toggle),
            Self::Choice { .. } => Some(Edit::NextChoice),
            Self::Number {
                value,
                min,
                max,
                step,
            } => Some(Edit::Step(if value >= max { min - value } else { step })),
            Self::Color(_) | Self::Keybind(_) => None,
        }
    }
}

fn setting(
    group: &'static str,
    label: &'static str,
    description: &'static str,
    value: SettingValue,
) -> Setting {
    Setting {
        group,
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
            description: "How Sherlock starts, and what it does once you've found something.",
            settings: vec![
                setting(
                    "startup",
                    "Launch on startup",
                    "Start the Sherlock daemon when you log in.",
                    Toggle(true),
                ),
                setting(
                    "startup",
                    "Close after run",
                    "Hide the launcher after an item was executed.",
                    Toggle(true),
                ),
                setting(
                    "behaviour",
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
            description: "Colors and density. Applied the next time the launcher opens.",
            settings: vec![
                setting(
                    "theme",
                    "Theme",
                    "Color scheme of the launcher.",
                    Choice {
                        options: &["System", "Dark", "Light"],
                        selected: 0,
                    },
                ),
                setting(
                    "theme",
                    "Accent color",
                    "Used for selections and highlights.",
                    Color(rgb(0xf5a524).into()),
                ),
                setting(
                    "theme",
                    "Background",
                    "Window background.",
                    Color(rgb(0x0e0e12).into()),
                ),
                setting(
                    "results",
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
            title: "Keyboard",
            description: "Every action has a key. Click one to record a new combination.",
            settings: vec![
                setting(
                    "navigation",
                    "Next item",
                    "Move the selection down.",
                    keys(&["ctrl", "n"]),
                ),
                setting(
                    "navigation",
                    "Previous item",
                    "Move the selection up.",
                    keys(&["ctrl", "p"]),
                ),
                setting(
                    "navigation",
                    "Context menu",
                    "Open actions for the selected item.",
                    keys(&["ctrl", "l"]),
                ),
                setting("window", "Quit", "Close the launcher.", keys(&["escape"])),
            ],
        },
        SettingsSection {
            title: "Search",
            description: "How results are matched and ranked. Picks stay on this device.",
            settings: vec![
                setting(
                    "ranking",
                    "Learn from picks",
                    "Rank items you pick often higher for the same query.",
                    Toggle(true),
                ),
                setting(
                    "ranking",
                    "Fuzzy matching",
                    "Also match slightly misspelled queries.",
                    Toggle(false),
                ),
            ],
        },
    ]
}

const NAV_WIDTH: f32 = 208.;
/// Left/right padding of the content; rows reach `ROW_PAD` further out so their selection
/// background has room while their text stays aligned with the title.
const CONTENT_PAD: f32 = 36.;
const ROW_PAD: f32 = 12.;

/// Settings in Sherlock's own look, below the shell's search bar: numbered sections on the left, a ruled list of settings with `// group` headings on the right, and a status
/// footer with unsaved changes and key hints. The search filters settings across sections.
///
/// ↑↓ pick a row, ←→ switch sections, Enter changes the selected value, backspace on an empty
/// search goes back.
pub struct SettingsView {
    sections: Vec<SettingsSection>,
    /// Values as loaded; a setting that differs is "modified".
    defaults: Vec<SettingsSection>,
    /// Index of the shown section (while not searching).
    active: usize,
    /// Selected position in `visible()`.
    row: usize,
    /// Lowercased search text; empty: show the active section.
    query: SharedString,
    /// Keeps the selected row in view.
    scroll: ScrollHandle,
}

impl SettingsView {
    pub fn new() -> Self {
        let sections = dummy_sections();
        Self {
            defaults: sections.clone(),
            sections,
            active: 0,
            row: 0,
            query: SharedString::default(),
            scroll: ScrollHandle::new(),
        }
    }

    fn searching(&self) -> bool {
        !self.query.is_empty()
    }

    /// `(section, row)` of every listed setting: the active section, or all matches while
    /// searching.
    fn visible(&self) -> Vec<(usize, usize)> {
        if !self.searching() {
            let n = self
                .sections
                .get(self.active)
                .map_or(0, |s| s.settings.len());
            return (0..n).map(|r| (self.active, r)).collect();
        }
        self.sections
            .iter()
            .enumerate()
            .flat_map(|(si, s)| {
                s.settings
                    .iter()
                    .enumerate()
                    .filter(|(_, st)| st.matches(&self.query))
                    .map(move |(ri, _)| (si, ri))
            })
            .collect()
    }

    fn is_modified(&self, section: usize, row: usize) -> bool {
        let value = |s: &[SettingsSection]| {
            s.get(section)
                .and_then(|s| s.settings.get(row))
                .map(|s| s.value.clone())
        };
        value(&self.sections) != value(&self.defaults)
    }

    fn unsaved(&self) -> usize {
        self.sections
            .iter()
            .enumerate()
            .map(|(si, s)| {
                (0..s.settings.len())
                    .filter(|&ri| self.is_modified(si, ri))
                    .count()
            })
            .sum()
    }

    fn select_row(&mut self, row: usize) {
        self.row = row.min(self.visible().len().saturating_sub(1));
        self.scroll.scroll_to_item(self.row);
    }

    fn select_section(&mut self, section: usize) {
        let section = section.min(self.sections.len().saturating_sub(1));
        if section != self.active {
            self.active = section;
            self.row = 0;
            self.scroll.set_offset(Default::default());
        }
    }

    /// Enter on the selected row.
    fn activate(&mut self) {
        let Some(&(si, ri)) = self.visible().get(self.row) else {
            return;
        };
        if let Some(edit) = self.sections[si].settings[ri].value.activate() {
            apply_edit(&mut self.sections, si, ri, edit);
        }
    }

    /// Click handler that applies `edit` to setting `row` of `section`, listed at `pos`.
    fn on_edit(
        cx: &mut Context<Self>,
        (section, row): (usize, usize),
        pos: usize,
        edit: Edit,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, _: &ClickEvent, _, cx| {
            apply_edit(&mut this.sections, section, row, edit);
            this.row = pos;
            cx.notify();
        })
    }

    fn render_nav(
        &self,
        current: usize,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mono = |text: String, color: Hsla| {
            div()
                .font_family(theme.monospace.clone())
                .text_size(px(11.))
                .text_color(color)
                .child(text)
        };
        div()
            .id("settings-nav")
            .flex_none()
            .w(px(NAV_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(1.))
            .px(px(10.))
            .py(px(14.))
            .border_r_1()
            .border_color(theme.border)
            .children(self.sections.iter().enumerate().map(|(i, section)| {
                let active = i == current;
                div()
                    .id(("settings-section", i))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(34.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .map(|this| {
                        if active {
                            this.bg(theme.bg_selected)
                                .text_color(theme.primary_text)
                                .font_weight(FontWeight::MEDIUM)
                        } else {
                            this.text_color(theme.secondary_text)
                                .hover(|s| s.bg(theme.bg_selected))
                        }
                    })
                    .child(mono(
                        format!("{:02}", i + 1),
                        if active {
                            theme.accent()
                        } else {
                            theme.tertiary_text
                        },
                    ))
                    .child(div().flex_1().child(section.title))
                    .child(mono(
                        section.settings.len().to_string(),
                        theme.tertiary_text,
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.select_section(i);
                        cx.notify();
                    }))
            }))
    }

    fn render_control(
        value: &SettingValue,
        at: (usize, usize),
        pos: usize,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = ("setting", at.0 * 1000 + at.1);
        match value {
            SettingValue::Toggle(on) => {
                toggle(id, *on, theme, Self::on_edit(cx, at, pos, Edit::Toggle)).into_any_element()
            }
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
                Self::on_edit(cx, at, pos, Edit::NextChoice),
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
                Self::on_edit(cx, at, pos, Edit::Step(-step)),
                Self::on_edit(cx, at, pos, Edit::Step(*step)),
            )
            .into_any_element(),
        }
    }

    /// One setting, with its `// group` heading above it when the group starts here. The
    /// heading is part of the row so the scroll container holds rows only.
    fn render_row(
        &self,
        pos: usize,
        (si, ri): (usize, usize),
        heading: Option<SharedString>,
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let setting = &self.sections[si].settings[ri];
        let selected = pos == self.row;
        let mono = |text: SharedString, size: f32, color: Hsla| {
            div()
                .font_family(theme.monospace.clone())
                .text_size(px(size))
                .text_color(color)
                .child(text)
        };

        let row = div()
            .id(("settings-row", pos))
            .relative()
            .flex()
            .items_center()
            .gap(px(20.))
            .px(px(ROW_PAD))
            .py(px(14.))
            .rounded(px(8.))
            .map(|this| {
                if selected {
                    this.bg(theme.bg_selected)
                        .border_color(gpui::transparent_black())
                } else {
                    this.border_color(theme.border)
                }
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.select_row(pos);
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.primary_text)
                                    .child(setting.label),
                            )
                            .when(self.is_modified(si, ri), |this| {
                                this.child(mono("modified".into(), 10., theme.accent()))
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.secondary_text)
                            .child(setting.description),
                    ),
            )
            .child(div().flex_none().child(Self::render_control(
                &setting.value,
                (si, ri),
                pos,
                theme,
                cx,
            )));

        div()
            .flex()
            .flex_col()
            .when_some(heading, |this, heading| {
                this.child(
                    div()
                        .px(px(ROW_PAD))
                        .when(pos > 0, |this| this.pt(px(22.)))
                        .pb(px(4.))
                        .child(mono(heading, 11., theme.tertiary_text)),
                )
            })
            .child(row)
    }

    fn render_main(
        &self,
        visible: &[(usize, usize)],
        theme: &ThemeData,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, description): (SharedString, SharedString) = if self.searching() {
            let n = visible.len();
            (
                format!("“{}”", self.query).into(),
                format!("{n} setting{} found", if n == 1 { "" } else { "s" }).into(),
            )
        } else {
            self.sections
                .get(self.active)
                .map(|s| (s.title.into(), s.description.into()))
                .unwrap_or_default()
        };

        // `// group`, or `// section · group` while results come from several sections
        let mut last: Option<(usize, &str)> = None;
        let rows: Vec<AnyElement> = visible
            .iter()
            .enumerate()
            .map(|(pos, &(si, ri))| {
                let group = self.sections[si].settings[ri].group;
                let heading = (last != Some((si, group))).then(|| {
                    if self.searching() {
                        format!("// {} · {group}", self.sections[si].title.to_lowercase())
                    } else {
                        format!("// {group}")
                    }
                    .into()
                });
                last = Some((si, group));
                self.render_row(pos, (si, ri), heading, theme, cx)
                    .into_any_element()
            })
            .collect();

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(CONTENT_PAD))
                    .pt(px(26.))
                    .pb(px(22.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.primary_text)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.secondary_text)
                            .child(description),
                    ),
            )
            .child(
                div()
                    .id("settings-rows")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .flex()
                    .flex_col()
                    .px(px(CONTENT_PAD - ROW_PAD))
                    .pb(px(24.))
                    .children(rows),
            )
    }

    fn render_footer(&self, theme: &ThemeData) -> impl IntoElement {
        let unsaved = self.unsaved();
        let hint = |key: &'static str, label: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .px(px(5.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(theme.bg_keybind)
                        .text_color(theme.primary_text)
                        .child(key),
                )
                .child(label)
        };

        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(18.))
            .h(px(36.))
            .px(px(16.))
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.bg_status_bar)
            .font_family(theme.monospace.clone())
            .text_size(px(11.))
            .text_color(theme.secondary_text)
            .when(unsaved > 0, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_color(theme.accent())
                        .child(div().size(px(6.)).rounded(px(3.)).bg(theme.accent()))
                        .child(format!(
                            "{unsaved} unsaved change{}",
                            if unsaved == 1 { "" } else { "s" }
                        )),
                )
            })
            .child(div().flex_1())
            .child(hint("↑↓", "move"))
            .child(hint("←→", "section"))
            .child(hint("↵", "change"))
            .child(hint("⌫", "back"))
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
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

    /// The shell's search bar (it filters the settings); everything below it is drawn here.
    fn chrome(&self) -> Chrome {
        Chrome {
            search_bar: true,
            ..Chrome::NONE
        }
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        if query.text != self.query {
            self.query = query.text.clone();
            self.row = 0;
            self.scroll.set_offset(Default::default());
            cx.notify();
        }
        QueryResult::Ignored
    }

    /// ↑↓ rows, ←→ sections, Enter changes the selected value; back and quit keep the default.
    fn on_event(
        &mut self,
        event: &LauncherEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        match event {
            LauncherEvent::Move(MoveDirection::Up) => self.select_row(self.row.saturating_sub(1)),
            LauncherEvent::Move(MoveDirection::Down) => self.select_row(self.row + 1),
            LauncherEvent::Move(MoveDirection::Left) if !self.searching() => {
                self.select_section(self.active.saturating_sub(1))
            }
            LauncherEvent::Move(MoveDirection::Right) if !self.searching() => {
                self.select_section(self.active + 1)
            }
            LauncherEvent::Move(_) => {}
            LauncherEvent::Execute { .. } => self.activate(),
            _ => return Propagation::Continue,
        }
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
        let visible = self.visible();
        // while searching, the nav follows the selected result
        let current = match (self.searching(), visible.get(self.row)) {
            (true, Some(&(si, _))) => si,
            _ => self.active,
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg_app)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_nav(current, &theme, cx))
                    .child(self.render_main(&visible, &theme, cx)),
            )
            .child(self.render_footer(&theme))
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
    fn activate_steps_and_wraps_numbers() {
        let mut v = SettingValue::Number {
            value: 2,
            min: 1,
            max: 3,
            step: 1,
        };
        for expected in [3, 1, 2] {
            v.apply(v.activate().unwrap());
            assert!(matches!(v, SettingValue::Number { value, .. } if value == expected));
        }
    }

    #[test]
    fn activate_skips_values_with_own_editor() {
        assert!(SettingValue::Keybind(vec![]).activate().is_none());
        assert!(SettingValue::Color(rgb(0).into()).activate().is_none());
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
        assert_eq!(before, sections[0].settings[1].value);
    }

    #[test]
    fn search_spans_sections() {
        let mut view = SettingsView::new();
        view.query = "item".into();
        let sections: Vec<_> = view.visible().iter().map(|&(si, _)| si).collect();
        // "Close after run" (General) and three keybinds (Keyboard)
        assert!(sections.contains(&0) && sections.contains(&2));
        view.query = "".into();
        assert_eq!(view.visible().len(), view.sections[0].settings.len());
    }

    #[test]
    fn enter_marks_setting_modified_until_reverted() {
        let mut view = SettingsView::new();
        assert_eq!(view.unsaved(), 0);
        view.activate();
        assert!(view.is_modified(0, 0));
        assert_eq!(view.unsaved(), 1);
        view.activate();
        assert_eq!(view.unsaved(), 0);
    }
}
