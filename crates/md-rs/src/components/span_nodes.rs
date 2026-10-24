use std::borrow::Cow;

use md_rs_derive::{ComponentConstructor, SpanNode, TextCompomponent};

use super::span::Span;

#[derive(Default, SpanNode, ComponentConstructor, TextCompomponent)]
#[md_rs(rename = "p")]
pub struct Paragraph {
    pub spans: Vec<Span>,
}
impl Paragraph {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            spans: Vec::with_capacity(cap),
        }
    }
    pub fn span(mut self, span: Span) -> Self {
        self.spans.push(span);
        self
    }
}

#[macro_export]
macro_rules! p {
    ($($item:expr),* $(,)?) => {
        {
            const COUNT: usize = [$(stringify!($item)),*].len();
            let mut p = ::md_rs::components::span_nodes::Paragraph::with_capacity(COUNT);
            $(
                p = p.span($item.into());
            )*
            p
        }
    };
}

impl From<Span> for Paragraph {
    fn from(s: Span) -> Self {
        p!(s)
    }
}

impl From<&'static str> for Paragraph {
    fn from(s: &'static str) -> Self {
        p!(s)
    }
}

impl From<String> for Paragraph {
    fn from(s: String) -> Self {
        p!(s)
    }
}

impl From<Cow<'static, str>> for Paragraph {
    fn from(s: Cow<'static, str>) -> Self {
        p!(s)
    }
}

#[derive(Default, SpanNode, ComponentConstructor, TextCompomponent)]
#[span_node(prefix = "> ")]
pub struct Blockquote {
    pub spans: Vec<Span>,
}
