use std::fmt::{Result, Write};

use md_rs_derive::{ComponentBuilder, ComponentConstructor};

use crate::components::{Component, span_nodes::Paragraph};

#[derive(Default)]
pub enum AlertKind {
    #[default]
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "NOTE",
            Self::Tip => "TIP",
            Self::Important => "IMPORTANT",
            Self::Warning => "WARNING",
            Self::Caution => "CAUTION",
        }
    }
}

#[derive(Default, ComponentConstructor, ComponentBuilder)]
pub struct Alert {
    kind: AlertKind,
    text: Option<Paragraph>,
}

// kind impls
pub fn warning() -> Alert {
    Alert::default().kind(AlertKind::Warning)
}
pub fn note() -> Alert {
    Alert::default().kind(AlertKind::Note)
}
pub fn tip() -> Alert {
    Alert::default().kind(AlertKind::Tip)
}
pub fn important() -> Alert {
    Alert::default().kind(AlertKind::Important)
}
pub fn caution() -> Alert {
    Alert::default().kind(AlertKind::Caution)
}

impl Component for Alert {
    fn is_block(&self) -> bool {
        true
    }
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        let Some(text) = self.text.as_ref() else {
            return Ok(());
        };

        let mut buf = String::new();
        writeln!(buf, "> [!{}]", self.kind.as_str())?;
        text.render_inline(&mut buf)?;
        let mut first = true;
        for line in buf.lines() {
            if first {
                first = false;
                write!(out, "{line}")?;
            } else {
                write!(out, "\n> {line}")?;
            }
        }
        Ok(())
    }
}
