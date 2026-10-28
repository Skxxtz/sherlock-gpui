use md_rs::{
    components::{
        ParentComponentExt,
        TextComponentExt,
        container::Container,
        heading::h1,
        span_nodes::p,
    },
    github::components::{
        alert::{alert, warning},
        badge::badge,
        html_block::htmlblock,
        image::{image, source_dark, source_light},
    },
    md,
};

use crate::docs::{Documentation, Installation, contributing::ContributingShort};

pub(super) struct Readme;
impl Documentation for Readme {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            htmlblock()
                .orientation("center")
                .child(
                    image()
                        .source(source_dark("assets/logo-dark.svg"))
                        .source(source_light("assets/logo-light.svg"))
                        .src("assets/logo-light.svg")
                        .alt("sherlock logo")
                        .height("250"),
                )
                .child(
                    badge()
                        .alt("Discord")
                        .image_url(
                            "https://img.shields.io/discord/\
                            1357746313646833945.svg?color=7289da&&logo=discord",
                        )
                        .link("https://discord.gg/AQ44g4Yp9q"),
                )
                .child(
                    image()
                        .src("assets/mockup.png")
                        .width("100%")
                        .alt("application screenshot"),
                ),
            Description::docs(),
            h1("Getting Started"),
            Installation::docs(),
            ContributingShort::docs(),
            License::docs()
        )
    }
}

struct Description;
impl Documentation for Description {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            p().text("Sherlock is a")
                .bold("fast")
                .text(",")
                .bold("extensible")
                .text("application launcher for Wayland, build with")
                .link(
                    "GPUI",
                    "https://github.com/zed-industries/zed/tree/main/crates/gpui",
                )
                .text(".")
                .text("Sherlock's widgets inherit from launcher configurations.")
                .text("There are several launcher types, inlclugin a")
                .link("File Search", "") // TODO:
                .text(",")
                .link("Emoji Picker", "") // TODO:
                .text(", and")
                .link("Translator", "") // TODO:
                .text("."),
            alert().text(
                p().text("Sherlock has been rewritten entirely, to be compatible with")
                    .code("GPUI")
                    .text("instead of")
                    .code("GTK4")
                    .text(".")
                    .text(
                        "This included major refactorings, \
                        causing some changes to configuration files.",
                    ),
            ),
            warning().text(
                "Disclaimer: Due to GPUI's development primarily \
                focusing on Zed, some features may not be complete yet. \
                In Sherlock, this is barely noticeable though.",
            )
        )
    }
}

struct License;
impl Documentation for License {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h1("License"),
            p().text("GNU GENERAL LICENSE")
                .text("-")
                .text("see")
                .link(
                    "LICENSE",
                    "https://github.com/Skxxtz/sherlock/blob/main/LICENSE",
                )
                .text("for details."),
        )
    }
}
