use std::fmt::Write;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use chrono::Local;
use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, SharedString, Styled, div, prelude::FluentBuilder, px, rgb,
};
use simd_json::prelude::ArrayTrait;
use suite_223b::{
    calendar::utils::{
        CalDavEvent,
        structs::{Attendee, EventFilter, Partstat},
    },
    protocol::{Request, Response, SocketData},
    tokio::{AsyncSizedMessage, SizedMessageObj},
};

use crate::{
    app::{LAUNCH_GENERATION, theme::ThemeData},
    launcher::{LauncherConfig, utils::exec_mode::ExecMode, variant_type::LauncherType},
    loader::utils::{ApplicationAction, Priority},
    sherlock_msg,
    ui::{
        launcher::context_menu::ContextMenuAction,
        utils::async_update::{AsyncUpdate, AsyncUpdateEntity, Fetchable},
        widgets::{RenderableChildImpl, Selection},
    },
    utils::errors::{
        SherlockMessage,
        types::{SherlockErrorType, SocketAction},
    },
};

#[derive(Clone, Default, Debug)]
pub struct EventData {
    pub time: Option<SharedString>,
    pub event: Option<CalDavEvent>,
    pub color: Option<Hsla>,
}
impl Fetchable for EventData {
    type Error = SherlockMessage;
    async fn fetch(
        launcher: &Arc<LauncherConfig>,
        _old: Option<Rc<Self>>,
    ) -> Result<Option<Rc<Self>>, Self::Error> {
        let LauncherType::Event(evt) = &launcher.launcher_type else {
            unreachable!()
        };

        let mut stream = tokio::net::UnixStream::connect(SocketData::SOCKET_ADDR)
            .await
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Connect),
                    e
                )
            })?;

        let config = bincode::config::standard();
        let req = Request::Event(EventFilter::Nearby {
            look_back: evt.look_back,
            look_ahead: evt.look_ahead,
        });
        let req_obj = SizedMessageObj::from_struct(&req).map_err(|e| {
            sherlock_msg!(Warning, SherlockErrorType::SerializationError, e.message)
        })?;

        stream.write_sized(req_obj).await.map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Write),
                e.message
            )
        })?;

        let resp_bin = stream.read_sized().await.map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Read),
                e.message
            )
        })?;
        let (resp, _): (Response, _) = bincode::serde::decode_from_slice(&resp_bin, config)
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DeserializationError("Events".into()),
                    e
                )
            })?;

        if let Response::Events(mut events) = resp {
            let now = Local::now();

            events.sort_by(|a, b| {
                let a_start = a
                    .start_utc()
                    .map(|t| t.with_timezone(&Local))
                    .unwrap_or(now);
                let b_start = b
                    .start_utc()
                    .map(|t| t.with_timezone(&Local))
                    .unwrap_or(now);
                let a_end = a.end_utc().map(|t| t.with_timezone(&Local)).unwrap_or(now);
                let b_end = b.end_utc().map(|t| t.with_timezone(&Local)).unwrap_or(now);

                let a_is_active = now >= a_start && now < a_end;
                let b_is_active = now >= b_start && now < b_end;
                let a_is_upcoming = a_start > now;
                let b_is_upcoming = b_start > now;

                match (a_is_active, b_is_active) {
                    // both active: prefer the one ending soonest (most immediately relevant)
                    (true, true) => a_end.cmp(&b_end),

                    // one active, one not: active always wins
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,

                    // neither one is active: upcoming beats past
                    (false, false) => match (a_is_upcoming, b_is_upcoming) {
                        // both upcoming: soonest first
                        (true, true) => a_start.cmp(&b_start),

                        // one upcoming, one past: upcoming wins
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,

                        // both past: most recently ended first (least stale)
                        (false, false) => b_end.cmp(&a_end),
                    },
                }
            });

            let event = events.into_iter().next();
            let time = event.as_ref().and_then(|e| e.start_utc()).map(|utc_dt| {
                utc_dt
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string()
                    .into()
            });
            let color = event
                .as_ref()
                .and_then(|e| e.calendar_info.color.as_deref())
                .map(hex_to_u32)
                .map(rgb)
                .map(|s| s.into());

            Ok(Some(Rc::new(EventData { event, time, color })))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct EventWidget {
    pub actions: Arc<[Arc<ContextMenuAction>]>,
    pub entity: AsyncUpdateEntity<EventData>,
    last_call: Cell<Option<Instant>>,
    animation: Rc<Cell<AnimState>>,
    generation: Rc<Cell<u32>>,
}
impl EventWidget {
    pub fn new(cx: &mut impl AppContext) -> Self {
        Self {
            last_call: Cell::new(None),
            entity: AsyncUpdateEntity::<EventData>::new(cx),
            animation: Rc::new(Cell::new(AnimState::Inactive)),
            generation: Rc::new(Cell::new(0)),
            actions: Arc::from([]),
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
enum AnimState {
    #[default]
    Inactive,
    Done,
    InProgress,
}

impl<'a> RenderableChildImpl<'a> for EventWidget {
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        let Ok(Some(event_data)) = self.entity.read(cx) else {
            return div().into_any_element();
        };
        let Some(event) = event_data.event.as_ref() else {
            return div().into_any_element();
        };

        // Fix: fixed animation starting on new launcher generation
        let current_gen = LAUNCH_GENERATION.load(Ordering::Relaxed);
        let last_gen = self.generation.get();
        if current_gen != last_gen {
            self.animation.set(AnimState::Inactive);
            self.generation.set(current_gen);
        }

        let accent_color = event_data.color.unwrap_or(theme.bg_idle);
        div()
            .group("event-card")
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_start()
            .border_1()
            .rounded_md()
            .when(!selection.is_selected, |this| {
                this.border_color(theme.border_idle)
            })
            .child(
                div()
                    .size_full()
                    .flex()
                    .gap_5()
                    .items_center()
                    .child(
                        div()
                            .size(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().size(px(8.0)).rounded_full().bg(accent_color)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                // title and loc
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.primary_text)
                                            .child(event.title.clone()),
                                    )
                                    .when_some(event.location.clone(), |this, loc| {
                                        this.flex()
                                            .child(
                                                div()
                                                    .rounded_full()
                                                    .bg(theme.secondary_text)
                                                    .size(px(5.)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .font_family(theme.font_family.clone())
                                                    .text_color(theme.secondary_text)
                                                    .child(loc),
                                            )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_family(theme.font_family.clone())
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.secondary_text)
                                            .children(event_data.time.clone()),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .py_0()
                                            .rounded_sm()
                                            .bg(theme.bg_badge)
                                            .text_size(px(10.0))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.secondary_text)
                                            .child(event.calendar_info.name.clone()),
                                    ),
                            ),
                    ),
            )
            .when(
                !event.attendees.is_empty()
                    && (matches!(
                        self.animation.get(),
                        AnimState::InProgress | AnimState::Done
                    ) || selection.is_selected),
                |this| {
                    this.child(
                        div()
                            .px(px(44.))
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .border_color(theme.border_idle)
                            .child(
                                // A "Section Header" that looks like a tag
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_family(theme.font_family.clone())
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.secondary_text)
                                            .child("ATTENDEES"),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(theme.bg_selected)
                                            .text_size(px(9.))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.primary_text)
                                            .child(event.attendees.len().to_string()),
                                    ),
                            )
                            .child(
                                div().flex().flex_col().gap_1_5().children(
                                    event
                                        .attendees
                                        .iter()
                                        .take(8)
                                        .map(|att| render_attendee(att, &theme)),
                                ),
                            )
                            .with_animation(
                                if selection.is_selected {
                                    "attendee-reveal"
                                } else {
                                    "attendee-veal"
                                },
                                Animation::new(Duration::from_millis(200)).with_easing(SMOOTH_EASE),
                                {
                                    let anim_state = Rc::clone(&self.animation);
                                    let selection = selection.is_selected;
                                    move |this, delta| {
                                        let is_done = delta == 1.0;
                                        if is_done {
                                            if selection {
                                                anim_state.set(AnimState::Done);
                                            } else {
                                                anim_state.set(AnimState::Inactive);
                                            }
                                        } else {
                                            anim_state.set(AnimState::InProgress);
                                        }
                                        let delta = if selection { delta } else { 1.0 - delta };
                                        this.py(px(12. * delta))
                                            .mt(px(15. * delta))
                                            .opacity(delta)
                                            .max_h(px(delta * 200.))
                                            .border_t(px(delta))
                                            .occlude()
                                    }
                                },
                            ),
                    )
                },
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String> {
        let Ok(Some(event_outer)) = self.entity.read(cx) else {
            return None;
        };
        let event = event_outer.event.as_ref()?;

        let mut out = String::new();

        out.push_str(&event.title);
        match (event.start_utc(), event.end_utc()) {
            (Some(start), Some(end)) => {
                let _ = write!(
                    out,
                    "\n{} → {}",
                    start.with_timezone(&Local).format("%H:%M"),
                    end.with_timezone(&Local).format("%H:%M")
                );
            }
            (Some(start), None) => {
                let _ = write!(out, "{}", start.with_timezone(&Local).format("%H:%M"),);
            }
            _ => {}
        }

        Some(out)
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
    fn actions(
        &self,
        launcher: &Arc<LauncherConfig>,
        cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let event_data = self.entity.read(cx).as_ref().ok()?.as_ref()?;
        let event = event_data.event.as_ref()?;
        let meeting = event.meeting.as_ref();
        let extra = launcher.add_actions.as_ref();

        // early return if only actions apply
        if meeting.is_none() && extra.is_none_or(|e| e.is_empty()) {
            return Some(Arc::clone(&self.actions));
        }

        let mut cap = self.actions.len();
        if meeting.is_some() {
            cap += 1;
        }
        if let Some(e) = extra {
            cap += e.len();
        }

        let mut actions = Vec::with_capacity(cap);

        if let Some(url) = meeting.map(|m| m.url()) {
            actions.push(Arc::new(ContextMenuAction::App(
                ApplicationAction::new("inner.join_meeting", "Join Meeting")
                    .icon_name("call-start")
                    .exec(url.to_string()),
            )));
        }

        actions.extend(self.actions.iter().cloned());
        if let Some(extra_actions) = extra {
            actions.extend(extra_actions.iter().cloned());
        }

        Some(Arc::from(actions))
    }
    #[inline(always)]
    fn has_actions(&self, cx: &mut App) -> bool {
        self.entity.read(cx).as_ref().is_ok_and(|i| i.is_some())
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, cx: &mut C) -> Option<bool> {
        Some(self.entity.is_valid(cx))
    }
    fn update_async<C: AppContext>(&self, launcher: Arc<LauncherConfig>, cx: &mut C) {
        // debounce logic
        // causes freezes if not applied!!
        if let Some(last_call) = self.last_call.get()
            && last_call.elapsed() < Duration::from_secs(50)
        {
            return;
        }
        self.last_call.set(Some(Instant::now()));
        self.entity.update_async(launcher, cx);
    }
}

#[inline(always)]
fn hex_to_u32(hex: &str) -> u32 {
    let cleaned = hex.strip_prefix('#').unwrap_or(hex);
    u32::from_str_radix(cleaned, 16).unwrap_or(0)
}

fn render_attendee(attendee: &Attendee, theme: &ThemeData) -> impl IntoElement {
    let name = attendee.display_name.as_deref();
    let email = attendee.email.as_deref();

    let color = match attendee.partstat {
        Some(Partstat::Accepted) => theme.color_succ,
        Some(Partstat::Tentative) => theme.color_warn,
        Some(Partstat::Declined) => theme.color_err,
        _ => theme.secondary_text,
    };

    div()
        .flex()
        .flex_row()
        .justify_start()
        .items_center()
        .gap_2()
        .child(div().size(px(5.)).rounded_full().bg(color))
        .child(
            div()
                .flex()
                .flex_row()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_family(theme.font_family.clone())
                        .text_color(theme.primary_text)
                        .child(name.unwrap_or(email.unwrap_or("Unknown")).to_string()),
                )
                .when(name.is_some() && email.is_some(), |this| {
                    this.child(
                        div()
                            .text_size(px(12.0))
                            .font_family(theme.font_family.clone())
                            .text_color(theme.secondary_text.opacity(0.7))
                            .child(format!("({})", email.unwrap())),
                    )
                }),
        )
}

const SMOOTH_EASE: fn(f32) -> f32 = |t| {
    // This is a common "Ease-In-Out-Cubic" curve
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(3)) / 2.0
    }
};
