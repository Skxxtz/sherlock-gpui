use std::collections::HashMap;

use gpui::{AnyElement, Context, IntoElement, ScrollHandle, SharedString, Subscription, Window};

use crate::{
    ui::{
        model::data::NoData,
        views::{Chrome, Query, QueryResult, ShellCtx, SherlockView, settings::model::SectionKind},
    },
    utils::config::{ConfigGuard, SherlockConfig},
};

mod actions;
mod config_value;
mod control;
mod defs;
mod keymaps;
mod model;
mod render;
pub(crate) mod schema;
#[cfg(test)]
mod tests;

use control::Control;
use defs::{SectionDef, SettingDef};
use model::{Edit, Setting, SettingsSection};

pub struct SettingsView {
    /// What is shown and where each setting lives in the config.
    defs: Vec<SectionDef>,
    /// The config as edited: every change is written here first (saving writes it out).
    config: SherlockConfig,
    /// The values shown, read from `config`.
    sections: Vec<SettingsSection>,
    /// Values as loaded; a setting that differs is "modified".
    defaults: Vec<SettingsSection>,
    /// Editors of the settings that have one, by setting id.
    controls: HashMap<&'static str, Control>,
    /// Index of the shown section (while not searching).
    active: usize,
    /// Selected position in `visible()`.
    row: usize,
    /// Lowercased search text; empty: show the active section.
    query: SharedString,
    /// Keeps the selected row in view.
    scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

/// The config with every field at its default (what an empty config file gives).
fn default_config() -> SherlockConfig {
    toml::from_str("").unwrap_or_default()
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
        let theme = shell.theme.clone();
        self.render_page(&theme, cx).into_any_element()
    }
}

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = ConfigGuard::read()
            .map(|c| c.clone())
            .unwrap_or_else(|_| default_config());
        let defs = defs::sections();
        let credit_section = SettingsSection {
            title: "Credits",
            description: "The people behind Sherlock.",
            settings: vec![],
            kind: SectionKind::Credits,
        };
        let sections: Vec<_> = defs
            .iter()
            .map(|d| d.read(&config))
            .chain(std::iter::once(credit_section))
            .collect();

        let mut controls = HashMap::new();
        let mut subs = Vec::new();
        for setting in sections.iter().flat_map(|s| &s.settings) {
            if let Some((control, sub)) = Control::new(setting, cx) {
                controls.insert(setting.id, control);
                subs.push(sub);
            }
        }

        let this = Self {
            defs,
            config,
            defaults: sections.clone(),
            sections,
            controls,
            active: 0,
            row: 0,
            query: SharedString::default(),
            scroll: ScrollHandle::new(),
            _subs: subs,
        };
        this.refresh_keymap_notes(cx);
        this
    }

    /// Every setting, in order.
    fn settings(&self) -> impl Iterator<Item = &Setting> {
        self.sections.iter().flat_map(|s| &s.settings)
    }

    fn setting(&self, id: &str) -> Option<&Setting> {
        self.settings().find(|s| s.id == id)
    }

    fn setting_mut(&mut self, id: &str) -> Option<&mut Setting> {
        self.sections
            .iter_mut()
            .flat_map(|s| &mut s.settings)
            .find(|s| s.id == id)
    }

    fn def(&self, id: &str) -> Option<&SettingDef> {
        self.defs
            .iter()
            .flat_map(|s| &s.settings)
            .find(|d| d.id == id)
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

    /// The setting on the selected row.
    fn selected(&self) -> Option<&Setting> {
        let (si, ri) = *self.visible().get(self.row)?;
        self.sections.get(si)?.settings.get(ri)
    }

    fn is_modified(&self, id: &str) -> bool {
        let default = self
            .defaults
            .iter()
            .flat_map(|s| &s.settings)
            .find(|s| s.id == id);
        self.setting(id).map(|s| &s.value) != default.map(|s| &s.value)
    }

    fn unsaved(&self) -> usize {
        self.settings().filter(|s| self.is_modified(s.id)).count()
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

    /// Selects the row showing setting `id`, if it is listed.
    fn select_setting(&mut self, id: &str) {
        let visible = self.visible();
        if let Some(pos) = visible
            .iter()
            .position(|&(si, ri)| self.sections[si].settings[ri].id == id)
        {
            self.row = pos;
        }
    }

    /// The one place edits arrive (clicks, Enter, editors): applies `edit` to setting `id`,
    /// writes it to the config, and selects its row.
    fn apply(&mut self, id: &'static str, edit: Edit, cx: &mut Context<Self>) {
        let keybind = matches!(edit, Edit::SetChords(_));
        let Some(setting) = self.setting_mut(id) else {
            return;
        };
        setting.value.apply(edit);
        let value = setting.value.clone();
        if let Some(def) = self.def(id) {
            (def.set)(&mut self.config, &value);
        }
        self.reread(id, cx);
        self.select_setting(id);
        if keybind {
            self.refresh_keymap_notes(cx);
        }
        cx.notify();
    }

    /// Reads every setting back from the config, so writes that touch other settings show up
    /// (a chord moved from one action to another). Editors of changed settings are synced,
    /// except `edited`'s, which already shows its value.
    fn reread(&mut self, edited: &str, cx: &mut Context<Self>) {
        for (section, def) in self.sections.iter_mut().zip(&self.defs) {
            for (setting, def) in section.settings.iter_mut().zip(&def.settings) {
                let value = (def.get)(&self.config, def.meta);
                if value != setting.value {
                    if setting.id != edited
                        && let Some(control) = self.controls.get(setting.id)
                    {
                        control.sync(&value, cx);
                    }
                    setting.value = value;
                }
            }
        }
    }

    /// Enter on the selected row: opens its editor, or changes the value directly.
    fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(setting) = self.selected() else {
            return;
        };
        let id = setting.id;
        if let Some(control) = self.controls.get(id) {
            control.open(window, cx);
        } else if let Some(edit) = setting.value.activate() {
            self.apply(id, edit, cx);
        }
    }
}
