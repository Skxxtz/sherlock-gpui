use std::fmt::{Result, Write};

use md_rs_derive::{ComponentConstructor, ParentComponent};

use super::Component;
use crate::components::{ParentComponentExt, list::List};

#[derive(Default, ComponentConstructor, ParentComponent)]
pub struct Container {
    children: Vec<Box<dyn Component>>,
}
impl Container {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            children: Vec::with_capacity(cap),
        }
    }

    pub fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }

    pub fn list(self, f: impl FnOnce(List) -> List) -> Self {
        self.child(f(List::default()))
    }
}

impl Component for Container {
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        for child in &self.children {
            child.render(out)?;
        }

        Ok(())
    }
    fn render(&self, out: &mut dyn Write) -> Result {
        self.render_inline(out)
    }
}

#[macro_export]
macro_rules! md {
    ($($child:expr),* $(,)?) => {
        {
            const COUNT: usize = [$( { _ = stringify!($child); 1 } ),*].len();
            let mut container = ::md_rs::components::container::Container::with_capacity(COUNT);
            $(
                container = container.child($child);
            )*
            container
        }
    }
}
