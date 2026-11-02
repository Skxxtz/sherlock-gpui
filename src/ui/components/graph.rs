mod paint;
pub mod scale;

use std::sync::Arc;

use gpui::{
    App,
    DispatchPhase,
    ElementId,
    Hsla,
    IntoElement,
    MouseMoveEvent,
    ParentElement,
    Pixels,
    RenderOnce,
    SharedString,
    Styled,
    Window,
    canvas,
    div,
    prelude::FluentBuilder,
    px,
    relative,
};
use paint::Plot;
use scale::{Scale, format_tick};

use crate::app::theme::{ActiveTheme, ThemeData};

const AXIS_GAP: f32 = 6.;
const LABEL_SIZE: f32 = 10.;

#[allow(unused)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GraphKind {
    #[default]
    Line,
    Area,
    Bar,
    Scatter,
}

/// One named set of points. Non-finite values are skipped.
#[derive(Debug, Clone)]
pub struct Series {
    pub name: SharedString,
    pub points: Vec<(f64, f64)>,
    pub color: Option<Hsla>,
}

impl Series {
    pub fn new(
        name: impl Into<SharedString>,
        points: impl IntoIterator<Item = (impl Into<f64>, impl Into<f64>)>,
    ) -> Self {
        Self {
            name: name.into(),
            points: points
                .into_iter()
                .map(|(x, y)| (x.into(), y.into()))
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect(),
            color: None,
        }
    }

    /// `y` values at x = 0, 1, 2, …
    pub fn from_values(
        name: impl Into<SharedString>,
        values: impl IntoIterator<Item = impl Into<f64>>,
    ) -> Self {
        Self::new(
            name,
            values.into_iter().enumerate().map(|(i, v)| (i as f64, v)),
        )
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

pub type Formatter = Arc<dyn Fn(f64) -> SharedString>;

#[derive(IntoElement)]
pub struct Graph {
    kind: GraphKind,
    series: Vec<Series>,
    height: Pixels,
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    ticks: usize,
    x_format: Option<Formatter>,
    y_format: Option<Formatter>,
    axes: bool,
    grid: bool,
    legend: Option<bool>,
    stroke: Pixels,
    /// Enables hover (guide line, dots, tooltip). Needs a unique id per graph.
    id: Option<ElementId>,
}

#[allow(unused)]
impl Graph {
    pub fn new(kind: GraphKind) -> Self {
        Self {
            kind,
            series: Vec::new(),
            height: px(120.),
            x_range: None,
            y_range: None,
            ticks: 4,
            x_format: None,
            y_format: None,
            axes: true,
            grid: true,
            legend: None,
            stroke: px(2.),
            id: None,
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn series_many(mut self, series: impl IntoIterator<Item = Series>) -> Self {
        self.series.extend(series);
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    /// Fixed x domain instead of fitting the data.
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.x_range = Some((min, max));
        self
    }

    /// Fixed y domain instead of fitting (and rounding) the data.
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }

    /// Approximate number of tick intervals per axis.
    pub fn ticks(mut self, ticks: usize) -> Self {
        self.ticks = ticks.max(1);
        self
    }

    /// Label for x ticks, e.g. index → weekday name.
    pub fn x_format(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.x_format = Some(Arc::new(move |v| f(v).into()));
        self
    }

    pub fn y_format(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.y_format = Some(Arc::new(move |v| f(v).into()));
        self
    }

    /// Show axis labels (default on). Off gives a bare sparkline.
    pub fn axes(mut self, show: bool) -> Self {
        self.axes = show;
        self
    }

    pub fn grid(mut self, show: bool) -> Self {
        self.grid = show;
        self
    }

    /// Defaults to shown when there's more than one series.
    pub fn legend(mut self, show: bool) -> Self {
        self.legend = Some(show);
        self
    }

    pub fn stroke(mut self, width: Pixels) -> Self {
        self.stroke = width;
        self
    }

    /// Unique id; turns on hover with a guide line, dots and a tooltip
    /// showing every series at the nearest x.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Strips everything but the data: no axes, grid or legend.
    pub fn sparkline(self) -> Self {
        self.axes(false).grid(false).legend(false)
    }

    fn colors(&self, theme: &ThemeData) -> Vec<Hsla> {
        let palette = [
            theme.accent(),
            theme.color_info,
            theme.color_succ,
            theme.color_err,
            theme.border_selected,
        ];
        self.series
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or(palette[i % palette.len()]))
            .collect()
    }

    /// Smallest gap between distinct x values; the width one bar group may take.
    fn bar_slot(&self) -> f64 {
        let mut xs: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.0))
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        let gap = xs
            .windows(2)
            .map(|w| w[1] - w[0])
            .fold(f64::INFINITY, f64::min);
        if gap.is_finite() && gap > 0. { gap } else { 1. }
    }
}

impl RenderOnce for Graph {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let colors = self.colors(&theme);
        let points = || self.series.iter().flat_map(|s| s.points.iter());
        let bars = self.kind == GraphKind::Bar;

        // x: exact data extent (bars get half a slot of padding each side)
        let slot = if bars { self.bar_slot() } else { 0. };
        let x = match self.x_range {
            Some((lo, hi)) => Scale::new(lo, hi),
            None => Scale::fit(points().map(|p| p.0), false).pad(slot / 2.),
        };
        let (x_ticks, x_step) = x.ticks(self.ticks);

        // y: rounded out to nice ticks unless fixed
        let (y, y_ticks, y_step) = match self.y_range {
            Some((lo, hi)) => {
                let y = Scale::new(lo, hi);
                let (ticks, step) = y.ticks(self.ticks);
                (y, ticks, step)
            }
            None => {
                let fit = Scale::fit(points().map(|p| p.1), bars);
                // Lines/areas: keep the extremes off the edges. Flat data gets room too.
                let fit = if bars {
                    fit
                } else {
                    let (lo, hi) = points()
                        .map(|p| p.1)
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
                            (a.min(v), b.max(v))
                        });
                    let span = hi - lo;
                    fit.pad(if span > 0. {
                        span * 0.15
                    } else {
                        lo.abs().max(1.) * 0.1
                    })
                };
                fit.nice(self.ticks)
            }
        };

        let fmt = |f: &Option<Formatter>, v: f64, step: f64| -> SharedString {
            match f {
                Some(f) => f(v),
                None => format_tick(v, step).into(),
            }
        };
        let x_labels: Vec<_> = x_ticks
            .iter()
            .map(|&v| (x.norm(v) as f32, fmt(&self.x_format, v, x_step)))
            .collect();
        let y_labels: Vec<_> = y_ticks
            .iter()
            .map(|&v| (1. - y.norm(v) as f32, fmt(&self.y_format, v, y_step)))
            .collect();

        let mut xs: Vec<f64> = points().map(|p| p.0).collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        let xs_norm: Vec<f32> = xs.iter().map(|v| x.norm(*v) as f32).collect();

        let hover_state = self
            .id
            .clone()
            .map(|id| window.use_keyed_state(id, cx, |_, _| None::<usize>));
        let hovered = hover_state
            .as_ref()
            .and_then(|s| *s.read(cx))
            .and_then(|i| xs.get(i).copied())
            .and_then(|xv| {
                let values: Vec<_> = self
                    .series
                    .iter()
                    .zip(&colors)
                    .filter_map(|(s, c)| {
                        let (_, v) = s.points.iter().find(|p| p.0 == xv)?;
                        Some((
                            s.name.clone(),
                            *c,
                            1. - y.norm(*v) as f32,
                            fmt(&self.y_format, *v, y_step),
                        ))
                    })
                    .collect();
                Some((x.norm(xv) as f32, fmt(&self.x_format, xv, x_step), values))
            });

        let n = self.series.len().max(1) as f64;
        let plot = Plot {
            kind: self.kind,
            x,
            y,
            y_ticks,
            series: self
                .series
                .iter()
                .zip(&colors)
                .map(|(s, c)| (s.points.clone(), *c))
                .collect(),
            grid: self.grid.then_some(theme.border_idle),
            stroke: self.stroke,
            bar_width: slot * 0.8 / n,
        };

        let label = |text: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .font_family(theme.typography.monospace.clone())
                .text_size(px(LABEL_SIZE))
                .text_color(theme.tertiary_text)
                .child(text)
        };

        let overlay = hovered.map(|(at, x_text, values)| {
            let guide = div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(at))
                .w(px(1.))
                .bg(theme.border_selected);
            let dots: Vec<_> = values
                .iter()
                .map(|(_, color, top, _)| {
                    // zero-size anchor centred on the point
                    div()
                        .absolute()
                        .left(relative(at))
                        .top(relative(*top))
                        .size_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .flex_none()
                                .size(px(7.))
                                .rounded_full()
                                .border_1()
                                .border_color(theme.bg_app)
                                .bg(*color),
                        )
                })
                .collect();
            // flip to the left side past the middle so it stays inside
            let tooltip = div()
                .absolute()
                .top_0()
                .when(at < 0.5, |d| d.left(relative(at)).ml(px(8.)))
                .when(at >= 0.5, |d| d.right(relative(1. - at)).mr(px(8.)))
                .flex()
                .flex_col()
                .gap(px(2.))
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .border_1()
                .border_color(theme.border_idle)
                .bg(theme.bg_overlay)
                .child(label(x_text))
                .children(values.into_iter().map(|(name, color, _, value)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(div().flex_none().size(px(6.)).rounded(px(1.)).bg(color))
                        .child(label(name))
                        .child(label(value).text_color(theme.secondary_text))
                }));
            (guide, dots, tooltip)
        });

        let plot_area = div()
            .relative()
            .flex_1()
            .h(self.height)
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        plot.paint(bounds, window);
                        let Some(state) = hover_state else { return };
                        let xs_norm = xs_norm.clone();
                        window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            let idx = bounds
                                .contains(&ev.position)
                                .then(|| {
                                    let frac =
                                        (ev.position.x - bounds.origin.x) / bounds.size.width;
                                    xs_norm
                                        .iter()
                                        .enumerate()
                                        .min_by(|a, b| {
                                            (a.1 - frac).abs().total_cmp(&(b.1 - frac).abs())
                                        })
                                        .map(|(i, _)| i)
                                })
                                .flatten();
                            state.update(cx, |s, cx| {
                                if *s != idx {
                                    *s = idx;
                                    cx.notify();
                                }
                            });
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .when_some(overlay, |this, (guide, dots, tooltip)| {
                this.child(guide).children(dots).child(tooltip)
            });

        // The y-axis is exactly as wide as its widest label: an invisible copy
        // of that label sits in normal flow, the real labels are absolute.
        // Monospace font, so "widest" = most characters.
        let widest = y_labels
            .iter()
            .map(|(_, t)| t.clone())
            .max_by_key(|t| t.chars().count())
            .unwrap_or_default();
        let gutter = |text: SharedString| label(text).h_0().invisible();
        // Same width as the y-axis, so x labels and legend line up with the plot.
        let indent = || {
            div()
                .flex_none()
                .when(self.axes, |this| this.child(gutter(widest.clone())))
        };

        // zero-size anchors centre each label on its tick
        let y_axis = div()
            .relative()
            .flex_none()
            .h(self.height)
            .child(gutter(widest.clone()))
            .children(y_labels.into_iter().map(|(at, text)| {
                div()
                    .absolute()
                    .right_0()
                    .top(relative(at))
                    .h_0()
                    .flex()
                    .items_center()
                    .child(label(text))
            }));

        let x_axis =
            div().flex().gap(px(AXIS_GAP)).child(indent()).child(
                div().relative().flex_1().h(px(LABEL_SIZE + 4.)).children(
                    x_labels.into_iter().map(|(at, text)| {
                        div()
                            .absolute()
                            .top_0()
                            .left(relative(at))
                            .w_0()
                            .flex()
                            .justify_center()
                            .child(label(text))
                    }),
                ),
            );

        let show_legend = self.legend.unwrap_or(self.series.len() > 1);
        let legend = div().flex().gap(px(AXIS_GAP)).child(indent()).child(
            div().flex().flex_1().flex_wrap().gap(px(10.)).children(
                self.series.iter().zip(colors).map(|(s, c)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(div().size(px(8.)).rounded(px(2.)).bg(c))
                        .child(label(s.name.clone()))
                }),
            ),
        );

        div()
            .flex()
            .flex_col()
            .w_full()
            .gap(px(4.))
            .child(
                div()
                    .flex()
                    .gap(px(AXIS_GAP))
                    .when(self.axes, |this| this.child(y_axis))
                    .child(plot_area),
            )
            .when(self.axes, |this| this.child(x_axis))
            .when(show_legend, |this| this.child(legend))
    }
}
