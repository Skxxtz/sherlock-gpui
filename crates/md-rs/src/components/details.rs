use std::fmt::{Result, Write};

use md_rs_derive::{ComponentBuilder, ComponentConstructor, ParentComponent};

use super::Component;
use crate::components::span_nodes::Paragraph;

#[derive(Default, ComponentConstructor, ParentComponent, ComponentBuilder)]
pub struct Details {
    summary: Paragraph,
    #[md_rs(skip_builder)]
    children: Vec<Box<dyn Component>>,
}

impl Details {
    pub fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
}

impl Component for Details {
    fn is_block(&self) -> bool {
        true
    }
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        writeln!(out, "<details>")?;
        write!(out, "<summary>")?;
        self.summary.render_inline(out)?;
        writeln!(out, "</summary>")?;
        writeln!(out)?; // blank line required by GFM after </summary>
        for child in &self.children {
            child.render(out)?; // each child self-terminates with \n
        }
        write!(out, "</details>") // no trailing \n
    }
}
