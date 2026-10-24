use std::fmt::{Result, Write};

use md_rs_derive::{ComponentBuilder, HeadingConstructors};

use super::Component;
use crate::components::span_nodes::Paragraph;

#[derive(Default, HeadingConstructors, ComponentBuilder)]
pub struct Heading {
    level: u8,
    #[md_rs(skip_builder)]
    text: Paragraph,
}

impl<C: Into<Paragraph>> From<C> for Heading {
    fn from(value: C) -> Self {
        Self {
            level: 1,
            text: value.into(),
        }
    }
}

impl Component for Heading {
    fn is_block(&self) -> bool {
        true
    }
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        let hashes = "#".repeat(self.level as usize);
        write!(out, "{hashes} ")?;
        self.text.render_inline(out)?;

        Ok(()) // no trailing \n
    }
}
