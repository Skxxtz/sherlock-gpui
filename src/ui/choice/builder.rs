use std::time::Duration;

use crate::ui::{
    choice::Choice,
    runtime_arguments::choice::ChoiceOption,
    utils::{pango::CachedPango, timeout::TimeoutCaller},
};
use gpui::Context;

pub struct ChoiceInputBuilder {
    scope: Option<&'static str>,
    placeholder: CachedPango,
    options: Option<Vec<ChoiceOption>>,
    selected: Option<usize>,
}

#[allow(dead_code)]
impl ChoiceInputBuilder {
    pub fn new() -> Self {
        Self {
            scope: None,
            placeholder: CachedPango::default(),
            options: None,
            selected: None,
        }
    }
    pub fn scope(mut self, scope: &'static str) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<CachedPango>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn options(mut self, options: Vec<ChoiceOption>) -> Self {
        self.options = Some(options);
        self
    }

    pub fn selected(mut self, idx: Option<usize>) -> Self {
        self.selected = idx;
        self
    }

    pub fn build(self, cx: &mut Context<Choice>) -> Choice {
        let mut cursor_timer = TimeoutCaller::new(false, cx);
        cursor_timer.repeat();
        cursor_timer.start(Duration::from_millis(500), cx, |visible, _| {
            *visible = !*visible;
        });

        Choice {
            scope: self.scope,
            focus_handle: cx.focus_handle(),
            _sub: None,
            options: self.options.unwrap_or_default(),
            placeholder: self.placeholder,
            selected: self.selected,
            is_opened: false,
            max_width: None,
        }
    }
}
