use std::collections::HashMap;

use gpui::{
    AnyElement, AppContext, Context, Entity, IntoElement, ScrollHandle, SharedString, Subscription,
    Window,
};

use crate::ui::{
    components::{
        color_picker::{ColorEvent, ColorItem},
        keymap_input::KeymapEvent,
        keymap_item::KeymapItem,
        select::{Select, SelectEvent},
    },
    model::data::NoData,
    views::{Chrome, Query, QueryResult, ShellCtx, SherlockView},
};

mod actions;
mod control;
mod data;
mod keymaps;
mod model;
mod render;
#[cfg(test)]
mod tests;

use model::{Edit, SettingValue, SettingsSection};

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

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
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
        self.render_page(&theme, cx).into_any_element()
    }
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
            model::apply_edit(&mut self.sections, si, ri, edit);
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
                        model::apply_edit(&mut this.sections, si, ri, Edit::SetColor(*color));
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
                        model::apply_edit(&mut this.sections, si, ri, Edit::SetChoice(*i));
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

    fn chords_mut(&mut self, section: usize, row: usize) -> Option<&mut Vec<SharedString>> {
        match &mut self.sections.get_mut(section)?.settings.get_mut(row)?.value {
            SettingValue::Keybind(chords) => Some(chords),
            _ => None,
        }
    }
}
