use std::{sync::Arc, time::Duration};

use gpui::{
    AnyElement, App, AppContext, Entity, IntoElement, ParentElement, SharedString, Styled, Task,
    div, px,
};
use smallvec::SmallVec;

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig, timer_launcher::TimerLauncherFunctions, utils::exec_mode::ExecMode,
        variant_type::InnerFunction,
    },
    loader::utils::{ExecVariable, Priority},
    ui::{
        launcher::context_menu::{ContextMenuAction, DynamicFunctionAction},
        utils::{search::SherlockSearch, timeout::Timeout},
        widgets::{
            RenderableChildImpl, Selection,
            timer::{
                model::{Timer, TimerState},
                render::{render_new_timer_pill, render_timer},
            },
        },
    },
    utils::intent::{Intent, cursor::Cursor, parsers::timer::TimerParser},
};

mod model;
mod render;

#[derive(Default)]
struct TimerEntity {
    intent: Option<Intent>,
    timers: SmallVec<[Timer; 4]>,
    update_task: Option<Task<()>>,
}

#[derive(Clone)]
pub struct TimerChild {
    update_entity: Entity<TimerEntity>,
    variable: [ExecVariable; 1],
}
impl TimerChild {
    pub fn new(cx: &mut App) -> Self {
        let update_entity = cx.new(|_| TimerEntity::default());
        let variable = [ExecVariable::Command(SharedString::from("command").into())];
        Self {
            update_entity,
            variable,
        }
    }
    pub fn toggle<C: AppContext>(&self, cx: &mut C) {
        self.update_entity.update(cx, |this, cx| {
            this.timers.iter_mut().for_each(|timer| timer.toggle(cx));
            cx.notify();
        });
    }
    pub fn new_timer<C: AppContext>(
        &self,
        duration: Duration,
        command: Option<SharedString>,
        cx: &mut C,
    ) {
        self.update_entity.update(cx, |this, cx| {
            if this.timers.len() < 4 {
                this.timers.push(Timer::new(duration, command, cx));
                cx.notify();
            }
        })
    }
}

impl<'a> RenderableChildImpl<'a> for TimerChild {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        _selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        let timers: SmallVec<[(TimerState, f32); 4]> = self.update_entity.update(cx, |this, cx| {
            if this.timers.iter().any(|t| t.state.is_running()) {
                // Will cause cx.notify to run
                this.start_timer(Duration::from_secs(1), cx, |_, _| {});
            }
            this.timers.retain(|t| t.state.remaining() > Duration::ZERO);
            this.timers.iter().map(|t| (t.state, t.amount)).collect()
        });
        let intent = self.update_entity.read(cx).intent.clone();

        let mut capacity = timers.len();
        if intent.is_some() {
            capacity += 1;
        }

        let mut children = Vec::with_capacity(capacity);
        for (state, initial_secs) in timers {
            children.push(render_timer(state, initial_secs, &theme));
        }
        if let Some(Intent::Timer { duration }) = intent
            && capacity - 1 != 4
        {
            children.push(render_new_timer_pill(duration, &theme))
        }

        if children.is_empty() {
            return div()
                .w_full()
                .px(px(16.0))
                .py(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .font_family(theme.font_family.clone())
                .text_color(theme.secondary_text)
                .child("No timers yet")
                .into_any_element();
        }

        div()
            .w_full()
            .px(px(16.0))
            .py(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .font_family(theme.font_family.clone())
            .children(children)
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        if let Some(Intent::Timer { duration }) = self.update_entity.read(cx).intent.clone() {
            return Some(ExecMode::Inner {
                func: InnerFunction::Timer(TimerLauncherFunctions::NewTimer { duration }),
                exit: false,
            });
        }
        Some(ExecMode::Inner {
            func: InnerFunction::Timer(TimerLauncherFunctions::Toggle),
            exit: false,
        })
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        None
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, keyword: &str, cx: &mut C) -> Option<bool> {
        let clean: SmallVec<[&str; 16]> = Intent::tokenize_kill_noise(keyword).collect();
        let cur = Cursor::new(&clean);
        let intent = TimerParser::parse_intent(cur);

        let show = matches!(&intent, Some(Intent::Timer { .. }));

        self.update_entity.update(cx, |this, _| {
            this.intent = intent;
        });

        if show {
            return Some(true);
        }

        if keyword.fuzzy_match("timer") {
            return Some(true);
        }

        Some(false)
    }
    #[inline(always)]
    fn vars(&self, cx: &mut App) -> Option<&[crate::loader::utils::ExecVariable]> {
        if let Some(Intent::Timer { .. }) = &self.update_entity.read(cx).intent {
            return Some(&self.variable);
        }
        None
    }
    #[inline(always)]
    fn has_actions(&self, cx: &mut App) -> bool {
        !self.update_entity.read(cx).timers.is_empty()
    }
    fn actions(
        &self,
        _launcher: &Arc<LauncherConfig>,
        cx: &mut App,
    ) -> Option<Arc<[Arc<crate::ui::launcher::context_menu::ContextMenuAction>]>> {
        // TODO: idea: make a context_menu action type parameter that allows for a binary option
        // like toggle or remove? this would allow a neater design rather than a remove and a
        // toggle option. could look like emoji column based action: |[option 1][option 2] Timer n|
        Some(
            (0..self.update_entity.read(cx).timers.len())
                .map(|i| {
                    Arc::new(ContextMenuAction::Fn(
                        DynamicFunctionAction::new(format!("Remove timer {}", i + 1))
                            .exit(false)
                            .icon_name("sherlock-process")
                            .on_exec({
                                let weak_self = self.update_entity.downgrade();
                                move |cx| {
                                    if let Some(this) = weak_self.upgrade() {
                                        this.update(cx, |this, _| {
                                            if i < this.timers.len() {
                                                this.timers.remove(i);
                                            }
                                        });
                                    }
                                }
                            }),
                    ))
                })
                .collect::<Arc<[_]>>(),
        )
    }
}

impl Timeout for TimerEntity {
    fn update_task(&self) -> &Option<Task<()>> {
        &self.update_task
    }
    fn update_task_mut(&mut self) -> &mut Option<Task<()>> {
        &mut self.update_task
    }
}
