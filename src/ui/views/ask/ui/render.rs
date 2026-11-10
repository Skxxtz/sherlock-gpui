use std::time::Duration;

use gpui::{
    Animation,
    AnimationExt,
    AnyElement,
    ClickEvent,
    Div,
    FontStyle,
    FontWeight,
    HighlightStyle,
    IntoElement,
    MouseButton,
    ParentElement,
    ScrollHandle,
    SharedString,
    StrikethroughStyle,
    Styled,
    StyledText,
    UnderlineStyle,
    WeakEntity,
    div,
    prelude::*,
    pulsating_between,
    px,
    relative,
};

use super::tokens::*;
use crate::{
    app::theme::ThemeData,
    ui::views::ask::{
        AskView,
        content::{
            actions::{AppEntry, Resolved, resolve_block},
            markdown::{self, Block, Inline, Rich},
        },
        exec::runner::{CommandRun, MAX_SHOWN_LINES, RunState, needs_terminal},
        model::{
            exchange::{Exchange, Phase},
            focus::Focus,
            items::{is_next_block, suggestion_lines},
        },
        store::history::ChatSummary,
        ui::selection::TextSelection,
    },
};

mod actions;
mod answer;
mod chrome;
mod code;
mod output;
mod suggestions;
mod table;
mod text;

use actions::*;
pub use answer::answer;
pub use chrome::{EmptyState, empty_state, progress_bar};
use chrome::{hint_el, kbd};
use code::*;
use output::*;
use suggestions::*;
use table::*;
use text::*;

/// Everything rendering needs, plus counters that number text blocks and runnable items in
/// document order.
pub struct Ui<'a> {
    pub theme: &'a ThemeData,
    pub view: WeakEntity<AskView>,
    pub apps: &'a [AppEntry],
    /// The answer is complete: runnable items can be selected.
    pub ready: bool,
    /// Keyboard focus: an item or its output panel.
    pub focus: Focus,
    pub text_sel: Option<&'a TextSelection>,
    /// Commands run from this answer, with their output.
    pub runs: &'a [CommandRun],
    pub output_scroll: &'a ScrollHandle,
    pub text_index: usize,
    pub code_index: usize,
    pub runnable_index: usize,
}

impl Ui<'_> {
    fn next_text_id(&mut self) -> usize {
        self.text_index += 1;
        self.text_index - 1
    }
}
