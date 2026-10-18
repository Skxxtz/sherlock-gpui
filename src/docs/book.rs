use std::{
    fmt::{Result, Write},
    path::Path,
};

use md_rs::components::Component;

use crate::{docs::Documentation, utils::string::TrimInPlace};

pub trait TopLevelEntry {
    type Summary: Component;
    fn summary() -> Self::Summary;
    fn summary_md() -> String {
        let mut out = String::new();
        let _ = Self::summary().render(&mut out);
        out.trim_in_place();
        out
    }
    fn children() -> impl Iterator<Item = BookEntry> + 'static;
}

#[derive(Default)]
pub struct BookEntry {
    pub title: &'static str,
    pub file: Option<&'static str>,
    pub render_fn: Option<fn() -> String>,
    pub children: Vec<BookEntry>,
}

impl BookEntry {
    #[cfg(all(test, feature = "docs"))]
    pub fn render(self) -> Vec<String> {
        let mut out = Vec::new();

        if let Some(render) = self.render_fn {
            out.push((render)());
        }

        for child in self.children {
            out.extend(child.render());
        }

        out
    }
    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        if let (Some(render), Some(file)) = (self.render_fn, self.file) {
            std::fs::write(dir.join(file), render())?;
        }
        for child in &self.children {
            child.write(dir)?;
        }
        Ok(())
    }
    pub fn of<D: Documentation>() -> Self {
        Self {
            render_fn: Some(D::docs_md),
            ..Default::default()
        }
    }
    pub fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }
    pub fn with_file(mut self, file: &'static str) -> Self {
        self.file = Some(file);
        self
    }
}

pub struct Book(pub Vec<BookEntry>);
impl Book {
    fn summary(&self) -> String {
        let mut out = String::from("# Summary\n\n");
        let _ = self.render_summary(&self.0, &mut out, 0);
        out
    }
    fn render_summary(&self, entries: &[BookEntry], out: &mut impl Write, depth: usize) -> Result {
        for entry in entries {
            let indent = "  ".repeat(depth);
            writeln!(
                out,
                "{}- [{}](./{})",
                indent,
                entry.title,
                entry.file.unwrap_or("#")
            )?;

            if !entry.children.is_empty() {
                self.render_summary(&entry.children, out, depth + 1)?;
            }
        }
        Ok(())
    }
    pub fn write<P: AsRef<Path>>(&self, dir: P) -> std::io::Result<()> {
        let dir = dir.as_ref();
        // Build summary
        let summary = self.summary();
        let summary_path = dir.join("SUMMARY.md");
        std::fs::write(summary_path, summary)?;

        // Build files
        for entry in &self.0 {
            entry.write(dir)?;
        }
        Ok(())
    }

    #[cfg(all(test, feature = "docs"))]
    pub fn render(self) -> Vec<String> {
        let mut out = Vec::from([self.summary()]);
        for entry in self.0 {
            out.extend(entry.render());
        }
        out
    }
}
