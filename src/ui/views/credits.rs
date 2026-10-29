use gpui::{
    App,
    Hsla,
    IntoElement,
    ParentElement,
    Styled,
    div,
    linear_color_stop,
    linear_gradient,
    prelude::*,
    px,
};

use super::Propagation;
use crate::{
    app::theme::ActiveTheme,
    ui::{
        components::sherlock_pixel,
        model::data::NoData,
        traits::HandleActions,
        views::SherlockView,
    },
};

/// One entry in the credits; will later be generated into `assets/contributors.json`.
pub struct Contributor {
    pub login: &'static str,
    pub name: &'static str,
    pub commits: u32,
}

/// Contributor list, created by gh action
const CONTRIBUTORS: &[Contributor] = include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/contributors.rs"
));

/// Height of the bottom fade-out, in px.
const FADE: f32 = 48.;
/// Maximum height of the scrolling contributor list, in px.
const LIST_MAX: f32 = 160.;

pub struct CreditView;

impl HandleActions for CreditView {
    const UNHANDLED: Propagation = Propagation::Continue;
}

impl SherlockView for CreditView {
    type Data = NoData;

    fn chrome(&self) -> super::Chrome {
        super::Chrome::NONE
    }

    fn render_body(
        &mut self,
        _shell: &super::ShellCtx,
        _win: &mut gpui::Window,
        _cx: &mut gpui::prelude::Context<Self>,
    ) -> gpui::AnyElement {
        Credits.into_any_element()
    }
}

#[derive(IntoElement)]
pub struct Credits;

impl RenderOnce for Credits {
    fn render(self, _win: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let accent = theme.accent();
        let muted = theme.tertiary_text;

        // `git shortlog -sn`-style rows: count, name, @login.
        let rows = CONTRIBUTORS.iter().map(|c| {
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .w(px(40.))
                        .flex()
                        .justify_end()
                        .text_color(accent)
                        .child(c.commits.to_string()),
                )
                .child(div().text_color(theme.primary_text).child(c.name))
                .child(div().text_color(muted).child(format!("@{}", c.login)))
        });

        let bg = theme.bg_app;
        let fade = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(FADE))
            .bg(linear_gradient(
                180.,
                linear_color_stop(Hsla { a: 0., ..bg }, 0.),
                linear_color_stop(bg, 1.),
            ));

        // Only the rows scroll; the fade sits over the list's bottom edge.
        let list = div()
            .relative()
            .max_h(px(LIST_MAX))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("credits")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pb(px(FADE)) // last row can scroll clear of the fade
                    .children(rows),
            )
            .child(fade);

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .font_family(theme.monospace.clone())
            .text_sm()
            .child(sherlock_pixel::sherlock_pixel_art(&theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(div().text_lg().text_color(accent).child("sherlock"))
                    .child(
                        div()
                            .text_color(theme.secondary_text)
                            .child("built by these people"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().pb_1().text_color(muted).child(format!(
                        "$ git shortlog -sn  # {} contributors",
                        CONTRIBUTORS.len()
                    )))
                    .child(list),
            )
            .child(div().text_color(muted).child("thank you."))
    }
}
