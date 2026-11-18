use std::sync::Arc;

use gpui::{
    AnyElement,
    App,
    FontWeight,
    InteractiveElement,
    IntoElement,
    ParentElement,
    SharedString,
    Styled,
    div,
    prelude::FluentBuilder,
    px,
};

use crate::{
    app::theme::ThemeData,
    launcher::utils::exec_mode::ExecMode,
    ui::{
        model::{
            item::ShellItem,
            list_data::{ListId, ListItem},
        },
        views::list::{RowCtx, RowRender},
    },
    utils::errors::{Pos, Remedy, SherlockMessage, SherlockMessageLevel},
};

type DismissFunction = Arc<dyn Fn(&mut App, ListId) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MessageChild {
    pub message: SherlockMessage,
    /// The kind's message: the row's text.
    headline: SharedString,
    /// Level, kind, message, context and cause, for the filter.
    search: SharedString,
    /// Source lines around a parse error's position, read once.
    excerpt: Option<Excerpt>,
    pub on_dismiss: Option<DismissFunction>,
    pub count: usize,
}

#[derive(Clone)]
struct Excerpt {
    /// (line number, text), ending with the error's line.
    lines: Vec<(u32, SharedString)>,
    col: u32,
}

impl Excerpt {
    /// The error's line and the one before it.
    fn read(path: &std::path::Path, pos: Pos) -> Option<Self> {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(path).ok()?;
        let first = pos.line.saturating_sub(1).max(1);
        let lines = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .enumerate()
            .map(|(i, l)| (i as u32 + 1, l))
            .skip_while(|(n, _)| *n < first)
            .take_while(|(n, _)| *n <= pos.line)
            .map(|(n, l)| (n, SharedString::from(l)))
            .collect::<Vec<_>>();
        (!lines.is_empty()).then_some(Self {
            lines,
            col: pos.col,
        })
    }
}

impl MessageChild {
    /// Shown to the user: logs the message now.
    pub fn new(mut message: SherlockMessage) -> Self {
        message.report();
        let headline: SharedString = message.kind.to_string().into();
        let search = format!(
            "{} {} {} {}",
            level_tag(message.level).to_lowercase(),
            message.kind.code().to_lowercase(),
            message.kind.name().to_lowercase(),
            message.to_string().to_lowercase()
        )
        .into();
        let excerpt = match message.kind.remedy() {
            Some(Remedy::OpenFile(path, Some(pos))) => Excerpt::read(&path, pos),
            _ => None,
        };
        Self {
            message,
            headline,
            search,
            excerpt,
            on_dismiss: None,
            count: 1,
        }
    }

    pub fn on_dismiss(mut self, f: impl Fn(&mut App, ListId) + Send + Sync + 'static) -> Self {
        self.on_dismiss = Some(Arc::new(f));
        self
    }
}

impl ListItem for MessageChild {
    fn search(&self) -> &str {
        &self.search
    }
}

impl ShellItem for MessageChild {
    /// Enter applies the kind's remedy (opens the file), else copies the report.
    fn exec(&self, _cx: &mut App) -> Option<ExecMode> {
        match self.message.kind.remedy() {
            Some(Remedy::OpenFile(path, _)) => Some(ExecMode::Command {
                exec: format!("xdg-open {}", shell_escape(&path.to_string_lossy())),
            }),
            None => Some(ExecMode::Copy {
                content: self.message.to_report(),
            }),
        }
    }
}

impl RowRender<ListId> for MessageChild {
    fn render_row(&self, row: &RowCtx<ListId>, _cx: &mut App) -> AnyElement {
        let theme = &row.theme;
        let msg = &self.message;
        let level = level_color(msg.level, theme);
        let selected = row.selected;
        let text = if selected {
            theme.primary_text
        } else {
            theme.secondary_text
        };

        // lvl · kind · message · at · ×count
        let line = div()
            .flex()
            .items_center()
            .gap(px(COL_GAP))
            .child(
                div()
                    .w(px(LVL_W))
                    .flex_none()
                    .font_weight(FontWeight::BOLD)
                    .text_color(level)
                    .child(level_tag(msg.level)),
            )
            .child(
                div()
                    .w(px(KIND_W))
                    .flex_none()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_color(text)
                    .child(msg.kind.code()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_color(text)
                    .child(self.headline.clone()),
            )
            .child(
                div()
                    .w(px(AT_W))
                    .flex_none()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_color(theme.tertiary_text)
                    .child(short_location(&msg.location)),
            )
            .child(
                div()
                    .w(px(COUNT_W))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .font_weight(FontWeight::BOLD)
                    .text_color(level)
                    .when(self.count > 1, |d| d.child(self.count.to_string())),
            );

        let row_el = div()
            .id(("message", row.id.0 as usize))
            .w_full()
            .px(px(ROW_PX))
            .py(px(7.))
            .font_family(theme.typography.monospace.clone())
            .text_size(px(12.))
            .line_height(px(16.))
            .border_b_1()
            .border_color(theme.border_idle)
            .child(line);

        if !selected {
            return row_el.into_any_element();
        }

        let steps = msg.context.iter().map(|s| (s.clone(), false));
        let cause = (!msg.cause.is_empty()).then(|| (msg.cause.clone(), true));
        let trace = steps
            .chain(cause)
            .enumerate()
            .map(|(depth, (step, is_cause))| {
                // Fixed-width boxes, not spaces and glyphs: alignment can't drift with the font.
                let branch = (depth > 0).then(|| {
                    div()
                        .flex_none()
                        .w(px(TREE_STEP))
                        .text_color(theme.tertiary_text)
                        .child("└─")
                });
                div()
                    .flex()
                    .pl(px(TREE_STEP * depth.saturating_sub(1) as f32))
                    .children(branch)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(if is_cause {
                                level
                            } else {
                                theme.secondary_text
                            })
                            .child(step),
                    )
            });

        let excerpt = self.excerpt.as_ref().map(|ex| {
            let gutter = |n: String| {
                div()
                    .flex_none()
                    .w(px(GUTTER_W))
                    .mr(px(8.))
                    .pr(px(8.))
                    .flex()
                    .justify_end()
                    .border_r_1()
                    .border_color(theme.border_idle)
                    .text_color(theme.tertiary_text)
                    .child(n)
            };
            let lines = ex.lines.iter().map(|(n, line)| {
                div()
                    .flex()
                    .child(gutter(n.to_string()))
                    .child(div().text_color(theme.secondary_text).child(line.clone()))
            });
            let caret =
                div()
                    .flex()
                    .child(gutter(String::new()))
                    .child(div().text_color(level).child(format!(
                        "{}^",
                        " ".repeat(ex.col.saturating_sub(1) as usize)
                    )));
            div()
                .mt(px(6.))
                .flex()
                .flex_col()
                .whitespace_nowrap()
                .children(lines)
                .child(caret)
        });

        let footer = msg.kind.hint().map(|hint| {
            div()
                .mt(px(6.))
                .flex()
                .gap(px(6.))
                .child(div().flex_none().text_color(theme.accent).child("hint"))
                .child(div().min_w_0().text_color(theme.tertiary_text).child(hint))
        });

        row_el
            .child(
                div()
                    .ml(px(DETAIL_ML))
                    .mt(px(8.))
                    .line_height(px(19.))
                    .flex()
                    .flex_col()
                    .children(trace)
                    .children(excerpt)
                    .children(footer),
            )
            .into_any_element()
    }
}

// Column widths, shared with the view's header row.
pub const LVL_W: f32 = 32.;
pub const KIND_W: f32 = 64.;
pub const AT_W: f32 = 180.;
pub const COUNT_W: f32 = 24.;
pub const COL_GAP: f32 = 12.;
pub const ROW_PX: f32 = 12.;
/// Details of an open row start under the message column.
const DETAIL_ML: f32 = LVL_W + KIND_W + 2. * COL_GAP;
/// Line-number column of a source excerpt.
const GUTTER_W: f32 = 40.;
/// One tree level: the `└─ ` branch box and the indent per depth.
const TREE_STEP: f32 = 22.;

pub fn level_tag(level: SherlockMessageLevel) -> &'static str {
    match level {
        SherlockMessageLevel::Error => "ERR",
        SherlockMessageLevel::Warning => "WRN",
        SherlockMessageLevel::Info => "INF",
    }
}

pub fn level_color(level: SherlockMessageLevel, theme: &ThemeData) -> gpui::Hsla {
    match level {
        SherlockMessageLevel::Error => theme.color_err,
        SherlockMessageLevel::Warning => theme.color_warn,
        SherlockMessageLevel::Info => theme.color_info,
    }
}

/// `src/loader/launcher_loader.rs:214` → `launcher_loader.rs:214`
fn short_location(location: &SharedString) -> SharedString {
    match location.rsplit_once('/') {
        Some((_, tail)) => tail.to_string().into(),
        None => location.clone(),
    }
}

/// Backslash-escapes what `split_as_command` would split or strip.
fn shell_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, ' ' | '\\' | '\'' | '"') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
