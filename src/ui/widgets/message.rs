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
    utils::errors::{SherlockMessage, SherlockMessageLevel},
};

type DismissFunction = Arc<dyn Fn(&mut App, ListId) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MessageChild {
    pub message: SherlockMessage,
    /// The kind's message: the row's text.
    headline: SharedString,
    /// Level, kind, message, context and cause, for the filter.
    search: SharedString,
    pub on_dismiss: Option<DismissFunction>,
    pub count: usize,
}

impl MessageChild {
    /// Shown to the user: logs the message now.
    pub fn new(mut message: SherlockMessage) -> Self {
        message.report();
        let headline: SharedString = message.kind.to_string().into();
        let search = format!(
            "{} {} {}",
            level_tag(message.level).to_lowercase(),
            message.kind.name().to_lowercase(),
            message.to_string().to_lowercase()
        )
        .into();
        Self {
            message,
            headline,
            search,
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
    /// Enter copies the message.
    fn exec(&self, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::Copy {
            content: self.message.to_string(),
        })
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
                    .child(msg.kind.name()),
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
                let branch = if depth == 0 {
                    String::new()
                } else {
                    format!("{}└─ ", "   ".repeat(depth - 1))
                };
                div()
                    .flex()
                    .child(
                        div()
                            .flex_none()
                            .text_color(theme.tertiary_text)
                            .child(branch),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .text_color(if is_cause {
                                level
                            } else {
                                theme.secondary_text
                            })
                            .child(step),
                    )
            });

        row_el
            .child(
                div()
                    .ml(px(LVL_W + COL_GAP))
                    .mt(px(8.))
                    .line_height(px(19.))
                    .flex()
                    .flex_col()
                    .children(trace),
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
