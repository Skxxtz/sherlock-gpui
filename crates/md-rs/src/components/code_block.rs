use std::{
    borrow::Cow,
    fmt::{Result, Write},
};

use md_rs_derive::ComponentConstructor;

use super::Component;

#[derive(Default, ComponentConstructor)]
pub struct CodeBlock {
    pub lang: Option<Cow<'static, str>>,
    pub content: Cow<'static, str>,

    #[cfg(feature = "validate-examples")]
    kind: verify::ExampleKind,
}
impl CodeBlock {
    #[cfg_attr(feature = "validate-examples", track_caller)]
    pub fn lang(mut self, val: impl Into<Cow<'static, str>>) -> Self {
        let lang: Cow<'static, str> = val.into();
        #[cfg(feature = "validate-examples")]
        {
            use std::panic::Location;
            self.kind = verify::ExampleKind::new(lang.as_ref(), Location::caller());
        }

        self.lang = Some(lang);
        self
    }

    pub fn content(mut self, val: impl Into<Cow<'static, str>>) -> Self {
        self.content = val.into();
        self
    }

    pub fn line(mut self, line: impl AsRef<str>) -> Self {
        let content = self.content.to_mut();
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line.as_ref());
        self
    }
}

impl Component for CodeBlock {
    fn is_block(&self) -> bool {
        true
    }
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        #[cfg(feature = "validate-examples")]
        self.kind.check(self.content.as_ref());

        let fence = "```";
        let lang = self.lang.as_deref().unwrap_or("");
        writeln!(out, "{fence}{lang}")?;
        writeln!(out, "{}", self.content.trim_end())?;
        write!(out, "{fence}")
    }
}

#[cfg(feature = "validate-examples")]
mod verify {
    use std::panic::Location;

    #[derive(Default)]
    pub(super) enum ExampleKind {
        Json(&'static Location<'static>),
        #[default]
        Plain,
    }

    impl ExampleKind {
        pub(super) fn check(&self, content: &str) {
            match self {
                Self::Json(loc) => {
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(content) {
                        panic!("invalid JSON example at {}: {e}\n{}", loc, content);
                    }
                }
                Self::Plain => {}
            }
        }

        pub fn new(lang: &str, loc: &'static Location<'static>) -> Self {
            match lang {
                "json" => Self::Json(loc),
                _ => Self::Plain,
            }
        }
    }
}
