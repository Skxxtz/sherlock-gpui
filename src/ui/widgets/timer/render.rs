use std::{sync::Arc, time::Duration};

use gpui::{
    AnyElement,
    FillOptions,
    FontWeight,
    IntoElement,
    ParentElement,
    Path,
    PathBuilder,
    Pixels,
    Point,
    Styled,
    canvas,
    div,
    px,
};

use crate::{app::theme::ThemeData, ui::widgets::timer::TimerState};

pub(super) fn build_arc_path(
    center: Point<Pixels>,
    r: Pixels,
    stroke_width: Pixels,
    percentage: f32,
) -> Option<Path<Pixels>> {
    let steps = 128;
    let cap_steps = 16;
    let end_angle = percentage * std::f32::consts::TAU;
    let r_outer = r + stroke_width * 0.5;
    let r_inner = r - stroke_width * 0.5;

    let mut builder =
        PathBuilder::default().with_style(gpui::PathStyle::Fill(FillOptions::non_zero()));

    // Outer edge forward
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = t * end_angle - std::f32::consts::FRAC_PI_2;
        let x = center.x + r_outer * angle.cos();
        let y = center.y + r_outer * angle.sin();
        if i == 0 {
            builder.move_to(Point { x, y });
        } else {
            builder.line_to(Point { x, y });
        }
    }

    // Round end cap
    let end_angle_abs = end_angle - std::f32::consts::FRAC_PI_2;
    for i in 0..=cap_steps {
        let t = i as f32 / cap_steps as f32;
        let cap_angle = end_angle_abs + t * std::f32::consts::PI;
        let x = center.x + r * end_angle_abs.cos() + (stroke_width * 0.5) * cap_angle.cos();
        let y = center.y + r * end_angle_abs.sin() + (stroke_width * 0.5) * cap_angle.sin();
        builder.line_to(Point { x, y });
    }

    // Inner edge backward
    for i in (0..=steps).rev() {
        let t = i as f32 / steps as f32;
        let angle = t * end_angle - std::f32::consts::FRAC_PI_2;
        let x = center.x + r_inner * angle.cos();
        let y = center.y + r_inner * angle.sin();
        builder.line_to(Point { x, y });
    }

    // Round start cap
    let start_angle = -std::f32::consts::FRAC_PI_2;
    for i in 0..=cap_steps {
        let t = i as f32 / cap_steps as f32;
        let cap_angle = start_angle + std::f32::consts::PI + t * std::f32::consts::PI;
        let x = center.x + r * start_angle.cos() + (stroke_width * 0.5) * cap_angle.cos();
        let y = center.y + r * start_angle.sin() + (stroke_width * 0.5) * cap_angle.sin();
        builder.line_to(Point { x, y });
    }

    builder.build().ok()
}

pub(super) fn build_track_path(
    center: Point<Pixels>,
    r: Pixels,
    stroke_width: Pixels,
) -> Option<Path<Pixels>> {
    // Full circle track — just reuse arc with 100%
    // but build it as a proper closed ring with no caps needed
    let steps = 128;
    let r_outer = r + stroke_width * 0.5;
    let r_inner = r - stroke_width * 0.5;

    let mut builder = PathBuilder::default();

    // Outer ring
    for i in 0..=steps {
        let angle = (i as f32 / steps as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let x = center.x + r_outer * angle.cos();
        let y = center.y + r_outer * angle.sin();
        if i == 0 {
            builder.move_to(Point { x, y });
        } else {
            builder.line_to(Point { x, y });
        }
    }

    // Inner ring backward (creates the hollow ring shape)
    for i in (0..=steps).rev() {
        let angle = (i as f32 / steps as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let x = center.x + r_inner * angle.cos();
        let y = center.y + r_inner * angle.sin();
        builder.line_to(Point { x, y });
    }

    builder.build().ok()
}

pub(super) fn render_timer(
    state: TimerState,
    initial_secs: f32,
    theme: &Arc<ThemeData>,
) -> AnyElement {
    let total_secs = state.remaining().as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let progress = (total_secs as f32 / initial_secs).clamp(0.0, 1.0);

    div()
        .relative()
        .size(px(120.0))
        .child(
            canvas(
                |bounds, _win, _cx| {
                    let center = bounds.center();
                    let r = px(50.0);
                    let stroke_width = px(8.0);

                    let track = build_track_path(center, r, stroke_width);
                    (center, r, stroke_width, track)
                },
                {
                    let theme = theme.clone();
                    move |_bounds, (center, r, stroke_width, track), window, _cx| {
                        if let Some(track) = track {
                            window.paint_path(track, theme.border_idle);
                        }
                        if let Some(arc) = build_arc_path(center, r, stroke_width, progress) {
                            window.paint_path(arc, theme.border_selected);
                        }
                    }
                },
            )
            .absolute()
            .inset(px(0.0)),
        )
        .child(
            // Text overlay, centered over the canvas
            div()
                .absolute()
                .inset(px(0.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family(theme.monospace.clone())
                        .text_size(px(25.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.primary_text)
                        .child(format!("{:02}", minutes)),
                )
                .child(
                    div()
                        .font_family(theme.monospace.clone())
                        .text_size(px(25.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.primary_text.opacity(0.3))
                        .child(":"),
                )
                .child(
                    div()
                        .font_family(theme.monospace.clone())
                        .text_size(px(25.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.primary_text)
                        .child(format!("{:02}", seconds)),
                ),
        )
        .into_any_element()
}

pub(super) fn render_new_timer_pill(duration: Duration, theme: &Arc<ThemeData>) -> AnyElement {
    fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    div()
        .ml_2()
        .flex()
        .gap_1()
        .items_center()
        .justify_center()
        .py_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .h_9()
                .aspect_square()
                .rounded_full()
                .child(
                    div()
                        .text_color(theme.secondary_text)
                        .font_family(theme.font_family.clone())
                        .text_size(px(20.))
                        .mb(px(1.))
                        .child("+"),
                )
                .bg(theme.bg_badge),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px(px(12.0))
                .py(px(6.0))
                .bg(theme.bg_badge)
                .rounded_full()
                .child(div().child("⏰"))
                .child(
                    div()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.primary_text)
                        .font_weight(FontWeight::BOLD)
                        .child(format_duration(duration)),
                ),
        )
        .into_any_element()
}
