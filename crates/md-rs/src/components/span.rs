use std::{borrow::Cow, fmt::Write};

use crate::{
    components::{IntoComponent, span_nodes::Paragraph},
    p,
};

pub struct LinkData {
    pub title: Vec<Span>,
    pub target: Cow<'static, str>,
}

pub enum Span {
    LineBreak,
    Text(Cow<'static, str>),
    Bold(Cow<'static, str>),
    Italic(Cow<'static, str>),
    Code(Cow<'static, str>),
    StrikeThrough(Cow<'static, str>),
    Underline(Cow<'static, str>),
    #[cfg(feature = "github")]
    HtmlStrong(Cow<'static, str>),
    #[cfg(feature = "github")]
    HtmlUnderline(Cow<'static, str>),
    #[cfg(feature = "github")]
    Keybind(Cow<'static, str>),
    Link(Box<LinkData>),
}
impl Span {
    pub fn render(&self, out: &mut dyn Write) -> std::fmt::Result {
        match self {
            Span::LineBreak => writeln!(out, "\n"),
            Span::Text(t) => write!(out, "{t}"),
            Span::Bold(t) => write!(out, "**{t}**"),
            Span::Italic(t) => write!(out, "_{t}_"),
            Span::StrikeThrough(t) => write!(out, "~~{t}~~"),
            Span::Underline(t) => write!(out, "<u>{t}</u>"),
            Span::Code(t) => write!(out, "`{t}`"),
            #[cfg(feature = "github")]
            Span::HtmlStrong(t) => write!(out, "<strong>{t}</strong>"),
            #[cfg(feature = "github")]
            Span::HtmlUnderline(t) => write!(out, "<ins>{t}</ins>"),
            #[cfg(feature = "github")]
            Span::Keybind(t) => write!(out, "<kbd>{t}</kbd>"),
            Span::Link(link) => {
                write!(out, "[")?;
                for (i, span) in link.title.iter().enumerate() {
                    if i > 0 && span.needs_space_before() {
                        write!(out, " ")?;
                    }
                    span.render(out)?;
                }
                write!(out, "]({})", link.target)
            }
        }
    }
    #[inline(always)]
    pub fn needs_space_before(&self) -> bool {
        match self {
            Span::LineBreak => true,
            Span::Text(t) => !t.starts_with([',', '.', '!', '?', ':', ';', ')']),
            _ => true,
        }
    }
}

impl From<&'static str> for Span {
    fn from(s: &'static str) -> Self {
        Span::Text(Cow::Borrowed(s))
    }
}

impl From<Cow<'static, str>> for Span {
    fn from(s: Cow<'static, str>) -> Self {
        Span::Text(s)
    }
}

impl From<String> for Span {
    fn from(s: String) -> Self {
        Span::Text(Cow::Owned(s))
    }
}

impl IntoComponent for Span {
    type Comp = Paragraph;
    fn into_component(self) -> Self::Comp {
        p!(self)
    }
}

pub fn br() -> Span {
    Span::LineBreak
}

pub fn text(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Text(s.into())
}

pub fn bold(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Bold(s.into())
}

pub fn italic(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Italic(s.into())
}

pub fn code(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Code(s.into())
}

pub fn strikethrough(s: impl Into<Cow<'static, str>>) -> Span {
    Span::StrikeThrough(s.into())
}

pub fn underline(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Underline(s.into())
}

pub fn link(title: impl Into<Cow<'static, str>>, target: impl Into<Cow<'static, str>>) -> Span {
    Span::Link(Box::new(LinkData {
        title: vec![Span::Text(title.into())],
        target: target.into(),
    }))
}

pub fn link_bold(
    title: impl Into<Cow<'static, str>>,
    target: impl Into<Cow<'static, str>>,
) -> Span {
    Span::Link(Box::new(LinkData {
        title: vec![Span::Bold(title.into())],
        target: target.into(),
    }))
}

pub fn linebreak() -> Span {
    Span::LineBreak
}

#[cfg(feature = "github")]
pub fn html_strong(s: impl Into<Cow<'static, str>>) -> Span {
    Span::HtmlStrong(s.into())
}

#[cfg(feature = "github")]
pub fn html_underline(s: impl Into<Cow<'static, str>>) -> Span {
    Span::HtmlUnderline(s.into())
}

#[cfg(feature = "github")]
pub fn keybind(s: impl Into<Cow<'static, str>>) -> Span {
    Span::Keybind(s.into())
}
