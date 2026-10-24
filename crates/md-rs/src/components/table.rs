use std::{
    borrow::Cow,
    fmt::{Result, Write},
};

use md_rs_derive::ComponentConstructor;

use super::Component;

#[derive(Default, ComponentConstructor)]
pub struct Table {
    headers: Vec<Cow<'static, str>>,
    rows: Vec<Vec<Cow<'static, str>>>,
}

impl Table {
    pub fn headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> Self {
        self.headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }

    pub fn row(mut self, row: impl IntoIterator<Item = impl Into<Cow<'static, str>>>) -> Self {
        self.rows.push(row.into_iter().map(|c| c.into()).collect());
        self
    }

    pub fn rows(
        mut self,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<Cow<'static, str>>>>,
    ) -> Self {
        for row in rows {
            self = self.row(row);
        }
        self
    }
}

fn render_cell(out: &mut dyn Write, content: &str) -> Result {
    for c in content.chars() {
        match c {
            '\n' => write!(out, "<br>")?,
            '|' => write!(out, "\\|")?,
            _ => write!(out, "{c}")?,
        }
    }
    Ok(())
}

impl Component for Table {
    fn is_block(&self) -> bool {
        true
    }
    fn render_inline(&self, out: &mut dyn Write) -> Result {
        if self.headers.is_empty() {
            return Ok(());
        }
        write!(out, "|")?;
        for h in &self.headers {
            write!(out, " {} |", h)?;
        }
        writeln!(out)?;
        write!(out, "|")?;
        for _ in &self.headers {
            write!(out, "---|")?;
        }
        writeln!(out)?;
        for row in &self.rows {
            write!(out, "|")?;
            for cell in row {
                write!(out, " ")?;
                render_cell(out, cell)?;
                write!(out, " |")?;
            }
            if row != self.rows.last().unwrap() {
                writeln!(out)?;
            }
        }
        Ok(()) // last row has no trailing \n — render() adds it
    }
}
