use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    ui::components::{stepper::stepper, toggle::toggle},
};

use super::model::{Edit, SettingValue, apply_edit};

const NAV_WIDTH: f32 = 208.;
/// Left/right padding of the content; rows reach `ROW_PAD` further out so their selection
/// background has room while their text stays aligned with the title.
const CONTENT_PAD: f32 = 36.;
const ROW_PAD: f32 = 12.;

impl super::SettingsView {
    pub fn render_page(&self, theme: &ThemeData, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.render_nav(current, theme, cx))
                    .child(self.render_main(&visible, theme, cx)),
            )
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
}
