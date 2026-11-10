use std::{any::TypeId, borrow::Cow, collections::HashMap, sync::LazyLock};

use md_rs::components::span::{Span, link};

use crate::docs::SherlockDocumentation;

/// TypeId → file of the page that documents it. Built once, on first use.
static LINKS: LazyLock<HashMap<TypeId, &'static str>> =
    LazyLock::new(|| SherlockDocumentation::book().index());

/// Markdown link to the page documenting `T`, or plain text if it has no page.
#[allow(unused)]
pub fn link_to<T: 'static>(text: impl Into<Cow<'static, str>>) -> Span {
    match LINKS.get(&TypeId::of::<T>()) {
        Some(&file) => link(text, file),
        None => Span::Text(text.into()),
    }
}
