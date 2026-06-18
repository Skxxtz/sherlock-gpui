use chrono::{Local, Timelike};
use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, FontWeight, Hsla, Image, ImageSource,
    IntoElement, ParentElement, Styled, div, img, linear_gradient, prelude::FluentBuilder, px,
    relative,
};
use std::{rc::Rc, sync::Arc, time::Duration};

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig, utils::exec_mode::ExecMode, variant_type::LauncherType,
        weather_launcher::WeatherData,
    },
    loader::utils::Priority,
    sherlock_msg,
    ui::{
        utils::{
            async_update::{AsyncUpdate, AsyncUpdateEntity, Fetchable},
            ease::Ease,
            render::ListItemBorder,
            selection::Selection,
            timeout::TimeoutCaller,
        },
        widgets::RenderableChildImpl,
    },
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

impl Fetchable for WeatherData {
    type Error = SherlockMessage;
    async fn fetch(
        launcher: &Arc<LauncherConfig>,
        _old: Option<Rc<Self>>,
    ) -> Result<Option<Rc<Self>>, Self::Error> {
        let LauncherType::Weather(wttr) = &launcher.launcher_type else {
            return Err(sherlock_msg!(
                Error,
                SherlockErrorType::InvalidLauncher,
                format!(
                    "Wrong launcher type.\nExpected: WeatherLauncher\nGot:{:?}",
                    &launcher.launcher_type
                )
            ));
        };
        WeatherData::fetch_async(wttr)
            .await
            .map(|d| Some(Rc::new(d)))
    }
}

#[derive(Clone)]
pub struct WeatherWidget {
    timeout: TimeoutCaller<()>,
    entity: AsyncUpdateEntity<WeatherData>,
}
impl WeatherWidget {
    pub fn new(cx: &mut App) -> Self {
        Self {
            timeout: TimeoutCaller::new((), cx),
            entity: AsyncUpdateEntity::<WeatherData>::new(cx),
        }
    }
}

impl<'a> RenderableChildImpl<'a> for WeatherWidget {
    const HANDLES_BORDERS: bool = true;
    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement {
        let show_datetime = if let LauncherType::Weather(wttr) = launcher.launcher_type.as_ref() {
            wttr.show_datetime
        } else {
            false
        };

        let now = Local::now();
        let time = now.time();
        if show_datetime {
            let secs_until_next_minute = 60 - now.second() as u64;
            self.timeout
                .start(Duration::from_secs(secs_until_next_minute), cx, |_, _| {});
        }

        let data_ref = match self.entity.read(cx) {
            Ok(data) => data,
            Err(_) => {
                return div()
                    .h(px(100.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_0()
                    .list_item_border(&theme, &selection)
                    .bg(theme.bg_muted)
                    .text_xs()
                    .font_family(theme.font_family.clone())
                    .text_color(theme.tertiary_text)
                    .child("Weather currently unavailable")
                    .into_any_element();
            }
        };
        let Some(data) = data_ref else {
            return div()
                .h(px(100.))
                .flex()
                .items_stretch()
                .gap(px(8.))
                .list_item_border(&theme, &selection)
                .with_animation(
                    "pulsate",
                    Animation::new(Duration::from_secs(1))
                        .with_easing(Ease::ease_throb)
                        .repeat(),
                    move |this, fac| this.bg(theme.bg_muted.opacity(fac)),
                )
                .into_any_element();
        };

        let is_init = data.init;
        let (p1, p2) = data.css.background(time, data.sunset, data.sunrise);
        let text_color: Hsla = data.css.color(time, data.sunset, data.sunrise).into();
        div()
            .h(px(100.))
            .flex()
            .items_stretch()
            .gap(px(8.))
            // Main weather card
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .px_4()
                    .py(px(12.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded_xl()
                    .bg(linear_gradient(135., p1, p2))
                    .child(
                        // Left — label + condition
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_color(text_color.opacity(0.6))
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .font_family(theme.font_family.clone())
                                    .child(data.format_str.clone()),
                            )
                            .child(
                                div()
                                    .text_color(text_color)
                                    .text_size(px(11.))
                                    .font_family(theme.font_family.clone())
                                    .child(data.condition.clone()),
                            ),
                    )
                    .child(
                        // Right — icon + temperature
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(if let Some(icon) = data.icon.as_ref() {
                                if let Some(svg) = icon.svg() {
                                    svg.size(px(36.))
                                        .text_color(theme.primary_text)
                                        .into_any_element()
                                } else {
                                    img(icon.clone()).size(px(36.)).into_any_element()
                                }
                            } else {
                                img(ImageSource::Image(Arc::new(Image::empty())))
                                    .size(px(36.))
                                    .into_any_element()
                            })
                            .child(
                                div()
                                    .text_color(text_color)
                                    .text_size(px(40.))
                                    .line_height(relative(1.))
                                    .font_weight(FontWeight::NORMAL)
                                    .font_family(theme.font_family.clone())
                                    .child(data.temperature.clone()),
                            )
                            .with_animation(
                                "weather_fade_in",
                                Animation::new(Duration::from_millis(300))
                                    .with_easing(|t| t * t * (3.0 - 2.0 * t)),
                                move |this, frac| {
                                    let opacity = if is_init { frac } else { 1.0 };
                                    this.opacity(opacity.clamp(0.0, 1.0))
                                },
                            ),
                    ),
            )
            .when(show_datetime, |this| {
                this.child(
                    div()
                        .h_full()
                        .aspect_square()
                        .rounded_xl()
                        .bg(p2.color.opacity(0.85))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_color(text_color)
                                .text_size(px(22.))
                                .line_height(relative(1.))
                                .font_weight(FontWeight::NORMAL)
                                .font_family(theme.font_family.clone())
                                .child(time.format("%H:%M").to_string()),
                        )
                        .child(
                            div()
                                .text_color(text_color.opacity(0.5))
                                .text_size(px(9.))
                                .line_height(relative(1.))
                                .font_weight(FontWeight::MEDIUM)
                                .font_family(theme.font_family.clone())
                                .child(now.format("%a %d").to_string()),
                        ),
                )
            })
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String> {
        let Ok(Some(entity_inner)) = self.entity.read(cx) else {
            return None;
        };

        if !entity_inner.init {
            return None;
        };

        Some(format!(
            "{}, {}, {}",
            entity_inner.location, entity_inner.condition, entity_inner.temperature
        ))
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn search(&self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn update_async<C: AppContext>(&self, launcher: Arc<LauncherConfig>, cx: &mut C) {
        self.entity.update_async(launcher, cx);
    }
}
