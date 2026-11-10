use std::{
    any::TypeId,
    collections::HashMap,
    fmt::{Result, Write},
    path::Path,
};

use md_rs::components::Component;

use crate::{docs::Documentation, snake_file, utils::string::TrimInPlace};

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

pub struct BookEntryBuilder {
    of: TypeId,
    title: Option<&'static str>,
    file: Option<&'static str>,
    render_fn: Option<fn() -> String>,
    children: Option<Vec<BookEntry>>,
}

impl BookEntryBuilder {
    pub fn new<D: Documentation + 'static>() -> Self {
        Self {
            of: TypeId::of::<D>(),
            title: None,
            file: None,
            render_fn: Some(D::docs_md),
            children: None,
        }
    }

    pub fn new_toplevel<T: TopLevelEntry + 'static>() -> Self {
        Self {
            of: TypeId::of::<T>(),
            title: None,
            file: None,
            render_fn: Some(T::summary_md),
            children: Some(T::children().collect()),
        }
    }

    pub fn new_generic<G: 'static>() -> Self {
        Self {
            of: TypeId::of::<G>(),
            title: None,
            file: None,
            render_fn: None,
            children: None,
        }
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn file(mut self, title: &'static str) -> Self {
        self.file = Some(title);
        self
    }

    pub fn render_fn(mut self, render_fn: fn() -> String) -> Self {
        self.render_fn = Some(render_fn);
        self
    }

    pub fn children(mut self, children: impl Iterator<Item = BookEntry>) -> Self {
        self.children = Some(children.collect());
        self
    }

    pub fn build(self) -> BookEntry {
        BookEntry {
            of: self.of,
            title: self.title.unwrap_or_default(),
            file: self.file,
            render_fn: self.render_fn,
            children: self.children.unwrap_or_default(),
        }
    }
}

pub struct BookEntry {
    of: TypeId,
    pub title: &'static str,
    pub file: Option<&'static str>,
    pub render_fn: Option<fn() -> String>,
    pub children: Vec<BookEntry>,
}

impl BookEntry {
    pub fn new<T: Documentation + 'static>() -> Self {
        BookEntryBuilder::new::<T>()
            .title(stringify!(T))
            .file(snake_file!(T))
            .build()
    }
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
}

pub struct Book(pub Vec<BookEntry>);
impl Book {
    pub(super) fn index(&self) -> HashMap<TypeId, &'static str> {
        fn walk(entries: &[BookEntry], map: &mut HashMap<TypeId, &'static str>) {
            for e in entries {
                if let Some(file) = e.file {
                    map.entry(e.of).or_insert(file);
                }
                walk(&e.children, map);
            }
        }
        let mut map = HashMap::new();
        walk(&self.0, &mut map);
        map
    }

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
