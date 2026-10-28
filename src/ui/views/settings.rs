use std::collections::HashMap;

use gpui::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    ui::{
        components::{
            color_picker::{ColorEvent, ColorItem},
            keybind::chord_label,
            keymap_input::KeymapEvent,
            keymap_item::KeymapItem,
            select::{Select, SelectEvent},
            stepper::stepper,
            toggle::toggle,
        },
        model::data::NoData,
        views::{Chrome, Query, QueryResult, ShellCtx, SherlockView},
    },
};

mod actions;
mod data;

/// The value of one setting; decides which control is shown.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Toggle(bool),
    Color(Hsla),
    /// Every chord bound to the action (`"ctrl-n"`, `"down"`); an action can have several.
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
    SetChoice(usize),
    Step(i64),
    SetColor(Hsla),
}

impl SettingValue {
    fn apply(&mut self, edit: Edit) {
        match (self, edit) {
            (Self::Toggle(on), Edit::Toggle) => *on = !*on,
            (Self::Choice { options, selected }, Edit::SetChoice(i)) => {
                *selected = i.min(options.len().saturating_sub(1));
            }
            (
                Self::Number {
                    value, min, max, ..
                },
                Edit::Step(delta),
            ) => *value = (*value + delta).clamp(*min, *max),
            (Self::Color(c), Edit::SetColor(new)) => *c = new,
            _ => {}
        }
    }

    /// What Enter does: flip, or step up (wrapping to `min` at `max`).
    /// `None` for values that open their own editor (color, keybind, choice).
    fn activate(&self) -> Option<Edit> {
        match *self {
            Self::Toggle(_) => Some(Edit::Toggle),
            Self::Number {
                value,
                min,
                max,
                step,
            } => Some(Edit::Step(if value >= max { min - value } else { step })),
            Self::Color(_) | Self::Keybind(_) | Self::Choice { .. } => None,
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

/// An action bound to `chords` (`["ctrl-n", "down"]`).
fn keys(chords: &[&'static str]) -> SettingValue {
    SettingValue::Keybind(chords.iter().map(|&c| SharedString::from(c)).collect())
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
    /// One color input per color setting, created on first render.
    colors: HashMap<(usize, usize), Entity<ColorItem>>,
    /// One select per choice setting, created on first render.
    selects: HashMap<(usize, usize), Entity<Select>>,
    /// One key-binding input per keybind setting, created on first render.
    keymaps: HashMap<(usize, usize), Entity<KeymapItem>>,
    _subs: Vec<Subscription>,
}

impl SettingsView {
    pub fn new() -> Self {
        let sections = data::dummy_sections();
        Self {
            defaults: sections.clone(),
            sections,
            active: 0,
            row: 0,
            query: SharedString::default(),
            scroll: ScrollHandle::new(),
            colors: HashMap::new(),
            selects: HashMap::new(),
            keymaps: HashMap::new(),
            _subs: Vec::new(),
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

    /// Creates the color inputs of all color settings (once); their picks apply live.
    fn ensure_colors(&mut self, cx: &mut Context<Self>) {
        if !self.colors.is_empty() {
            return;
        }
        for (si, section) in self.sections.iter().enumerate() {
            for (ri, setting) in section.settings.iter().enumerate() {
                let SettingValue::Color(color) = setting.value else {
                    continue;
                };
                let item = cx.new(|cx| ColorItem::new(color, cx));
                self._subs.push(cx.subscribe(
                    &item,
                    move |this, _, ColorEvent::Changed(color): &ColorEvent, cx| {
                        apply_edit(&mut this.sections, si, ri, Edit::SetColor(*color));
                        this.select_setting((si, ri));
                        cx.notify();
                    },
                ));
                self.colors.insert((si, ri), item);
            }
        }
    }

    /// Creates the selects of all choice settings (once); a pick applies right away.
    fn ensure_selects(&mut self, cx: &mut Context<Self>) {
        if !self.selects.is_empty() {
            return;
        }
        for (si, section) in self.sections.iter().enumerate() {
            for (ri, setting) in section.settings.iter().enumerate() {
                let SettingValue::Choice { options, selected } = setting.value else {
                    continue;
                };
                let options = options.iter().map(|&o| SharedString::from(o)).collect();
                let select = cx.new(|cx| Select::new(options, selected, cx));
                self._subs.push(cx.subscribe(
                    &select,
                    move |this, _, SelectEvent::Changed(i): &SelectEvent, cx| {
                        apply_edit(&mut this.sections, si, ri, Edit::SetChoice(*i));
                        this.select_setting((si, ri));
                        cx.notify();
                    },
                ));
                self.selects.insert((si, ri), select);
            }
        }
    }

    /// Selects the row showing setting `at`, if it is visible.
    fn select_setting(&mut self, at: (usize, usize)) {
        if let Some(pos) = self.visible().iter().position(|&v| v == at) {
            self.row = pos;
        }
    }

    /// Creates the key-binding inputs of all keybind settings (once) and keeps them in sync.
    fn ensure_keymaps(&mut self, cx: &mut Context<Self>) {
        if !self.keymaps.is_empty() {
            return;
        }
        for (si, section) in self.sections.iter().enumerate() {
            for (ri, setting) in section.settings.iter().enumerate() {
                let SettingValue::Keybind(chords) = &setting.value else {
                    continue;
                };
                let (label, chords) = (setting.label, chords.clone());
                let input = cx.new(|cx| KeymapItem::new(label, chords, cx));
                self._subs.push(
                    cx.subscribe(&input, move |this, input, _: &KeymapEvent, cx| {
                        let chords = input.read(cx).chords(cx).to_vec();
                        if let Some(c) = this.chords_mut(si, ri) {
                            *c = chords;
                        }
                        this.select_setting((si, ri));
                        this.refresh_keymap_notes(cx);
                        cx.notify();
                    }),
                );
                self.keymaps.insert((si, ri), input);
            }
        }
        self.refresh_keymap_notes(cx);
    }

    /// Tells each input which of its chords other actions use too.
    fn refresh_keymap_notes(&self, cx: &mut Context<Self>) {
        for (&at, input) in &self.keymaps {
            let notes: Vec<Option<String>> = match &self.sections[at.0].settings[at.1].value {
                SettingValue::Keybind(chords) => chords
                    .iter()
                    .map(|c| {
                        let others = self.conflicts(at, c);
                        (!others.is_empty()).then(|| format!("also {}", others.join(", ")))
                    })
                    .collect(),
                _ => continue,
            };
            input.update(cx, |input, cx| input.set_notes(notes, cx));
        }
    }

    fn chords_mut(&mut self, section: usize, row: usize) -> Option<&mut Vec<SharedString>> {
        match &mut self.sections.get_mut(section)?.settings.get_mut(row)?.value {
            SettingValue::Keybind(chords) => Some(chords),
            _ => None,
        }
    }

    /// "⌃ J is also bound to Previous item", for the first shared chord of a keybind setting.
    fn conflict_note(&self, section: usize, row: usize) -> Option<String> {
        let SettingValue::Keybind(chords) = &self.sections.get(section)?.settings.get(row)?.value
        else {
            return None;
        };
        chords.iter().find_map(|chord| {
            let others = self.conflicts((section, row), chord);
            (!others.is_empty()).then(|| {
                format!(
                    "{} is also bound to {}",
                    chord_label(chord),
                    others.join(", ")
                )
            })
        })
    }

    /// Other actions bound to `chord`, by label.
    fn conflicts(&self, at: (usize, usize), chord: &str) -> Vec<&'static str> {
        self.sections
            .iter()
            .enumerate()
            .flat_map(|(si, s)| {
                s.settings
                    .iter()
                    .enumerate()
                    .map(move |(ri, st)| ((si, ri), st))
            })
            .filter(|&(other, st)| {
                other != at
                    && matches!(&st.value, SettingValue::Keybind(c) if c.iter().any(|c| c == chord))
            })
            .map(|(_, st)| st.label)
            .collect()
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
                    .child(
                        div()
                            .flex_1()
                            .child(section.title)
                            .when(active, |this| this.text_color(theme.accent())),
                    )
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
        &self,
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
            SettingValue::Color(_) => match self.colors.get(&at) {
                Some(item) => item.clone().into_any_element(),
                None => div().into_any_element(),
            },
            SettingValue::Keybind(_) => match self.keymaps.get(&at) {
                Some(input) => input.clone().into_any_element(),
                None => div().into_any_element(),
            },
            SettingValue::Choice { .. } => match self.selects.get(&at) {
                Some(select) => select.clone().into_any_element(),
                None => div().into_any_element(),
            },
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
                    .child(match self.conflict_note(si, ri) {
                        Some(note) => div()
                            .text_size(px(12.))
                            .text_color(theme.color_warn)
                            .child(note),
                        None => div()
                            .text_size(px(12.))
                            .text_color(theme.secondary_text)
                            .child(setting.description),
                    }),
            )
            .child(div().flex_none().child(self.render_control(
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
        let unsaved = self.unsaved();
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
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(26.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.primary_text)
                                    .child(title),
                            )
                            .when(unsaved > 0, |this| {
                                this.child(
                                    div().text_size(px(12.)).text_color(theme.accent()).child(
                                        format!(
                                            "{unsaved} unsaved change{}",
                                            if unsaved == 1 { "" } else { "s" }
                                        ),
                                    ),
                                )
                            }),
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
            status_bar: true,
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

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.ensure_keymaps(cx);
        self.ensure_colors(cx);
        self.ensure_selects(cx);
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
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::rgb;

    #[test]
    fn toggle_flips() {
        let mut v = SettingValue::Toggle(false);
        v.apply(Edit::Toggle);
        assert!(matches!(v, SettingValue::Toggle(true)));
    }

    #[test]
    fn choice_is_clamped() {
        let mut v = SettingValue::Choice {
            options: &["a", "b"],
            selected: 0,
        };
        v.apply(Edit::SetChoice(5));
        assert!(matches!(v, SettingValue::Choice { selected: 1, .. }));
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
        let choice = SettingValue::Choice {
            options: &["a"],
            selected: 0,
        };
        assert!(choice.activate().is_none());
    }

    #[test]
    fn set_color_applies_to_colors_only() {
        let red: Hsla = rgb(0xff0000).into();
        let mut v = SettingValue::Color(rgb(0).into());
        v.apply(Edit::SetColor(red));
        assert_eq!(v, SettingValue::Color(red));
        let mut t = SettingValue::Toggle(true);
        t.apply(Edit::SetColor(red));
        assert_eq!(t, SettingValue::Toggle(true));
    }

    #[test]
    fn shared_chords_are_reported() {
        let mut view = SettingsView::new();
        view.chords_mut(2, 0).unwrap().push("ctrl-p".into()); // Previous item has it too
        assert_eq!(view.conflicts((2, 0), "ctrl-p"), ["Previous item"]);
        assert!(view.conflict_note(2, 0).unwrap().contains("Previous item"));
    }

    #[test]
    fn mismatched_edit_is_ignored() {
        let mut v = SettingValue::Toggle(true);
        v.apply(Edit::Step(1));
        assert!(matches!(v, SettingValue::Toggle(true)));
    }

    #[test]
    fn edit_targets_one_row() {
        let mut sections = data::dummy_sections();
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
