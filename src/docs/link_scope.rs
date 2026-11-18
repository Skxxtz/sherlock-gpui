//! Links between pages, resolved after everything is rendered.
//!
//! While the docs render, every book page registers its own type for its file, and
//! `Documentation::section` registers the embedded type on the page being rendered, at its
//! first heading (mdBook gives every heading an id). `link_to::<T>` only writes a placeholder.
//! Once all pages are rendered, `resolve` turns each placeholder into `file.md#heading`. No
//! list of "what's on which page" to keep up, nothing extra in the output.

use std::{
    any::{TypeId, type_name},
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use md_rs::components::span::{Span, link};

/// Placeholder link target: `@ref:` and 16 hex digits.
const REF: &str = "@ref:";
const REF_LEN: usize = REF.len() + 16;

thread_local! {
    static SCOPE: RefCell<Scope> = RefCell::default();
}

#[derive(Default)]
struct Scope {
    /// The book page being rendered; `None` outside the book (README, CONTRIBUTING).
    page: Option<&'static str>,
    /// Where each type or named item is documented.
    targets: HashMap<Ref, Target>,
    /// Placeholder id → what it links to, and its name for errors.
    refs: HashMap<u64, (Ref, &'static str)>,
}

/// Something a link can point at: a documented type, or a named item on a page (a command).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Ref {
    Type(TypeId),
    Name(&'static str),
}

struct Target {
    file: &'static str,
    anchor: Option<String>,
}

/// Markdown link to where `T` is documented. Resolved when the book is written.
pub fn link_to<T: 'static>(text: impl Into<std::borrow::Cow<'static, str>>) -> Span {
    placeholder(Ref::Type(TypeId::of::<T>()), type_name::<T>(), text)
}

/// Markdown link to the item registered as `name` (`register_name_here`), e.g. a command id.
pub fn link_to_name(name: &'static str, text: impl Into<std::borrow::Cow<'static, str>>) -> Span {
    placeholder(Ref::Name(name), name, text)
}

fn placeholder(
    target: Ref,
    name: &'static str,
    text: impl Into<std::borrow::Cow<'static, str>>,
) -> Span {
    let id = ref_id(target);
    SCOPE.with_borrow_mut(|s| s.refs.insert(id, (target, name)));
    link(text, format!("{REF}{id:016x}"))
}

/// Starts a fresh render: forgets every page, anchor and link.
pub(super) fn reset() {
    SCOPE.with_borrow_mut(|s| *s = Scope::default());
}

/// `id` is the page `file` itself. Wins over sections that embed the same type.
pub(super) fn register_page(id: TypeId, file: &'static str) {
    SCOPE.with_borrow_mut(|s| {
        s.targets
            .insert(Ref::Type(id), Target { file, anchor: None });
    });
}

/// Renders with `file` as the current page.
pub(super) fn in_page<R>(file: &'static str, render: impl FnOnce() -> R) -> R {
    let outer = SCOPE.with_borrow_mut(|s| s.page.replace(file));
    let out = render();
    SCOPE.with_borrow_mut(|s| s.page = outer);
    out
}

/// Registers `T` as documented on the current page, at the first heading of its `markdown`
/// (only rendered when needed). Does nothing outside the book, or if `T` is registered already
/// (the first place wins; a page registered with `register_page` always does).
pub(super) fn register_here<T: 'static>(markdown: impl FnOnce() -> String) {
    let id = Ref::Type(TypeId::of::<T>());
    let Some(page) = SCOPE.with_borrow(|s| s.page.filter(|_| !s.targets.contains_key(&id))) else {
        return;
    };
    // rendering may register nested sections: no borrow held meanwhile
    let anchor = heading_id(&markdown());
    SCOPE.with_borrow_mut(|s| {
        s.targets.entry(id).or_insert(Target { file: page, anchor });
    });
}

/// Registers `name` on the current page at the heading `heading` (markdown, `### …`), for
/// `link_to_name`. Does nothing outside the book; the first registration wins.
pub(super) fn register_name_here(name: &'static str, heading: &str) {
    SCOPE.with_borrow_mut(|s| {
        if let Some(file) = s.page {
            s.targets.entry(Ref::Name(name)).or_insert(Target {
                file,
                anchor: heading_id(heading),
            });
        }
    });
}

/// Replaces every placeholder in `text` with its target, `prefix` before the file (for
/// output outside the book: `docs/src/`). `Err`: the types linked to but documented nowhere.
pub(super) fn resolve(text: &str, prefix: &str) -> Result<String, Vec<&'static str>> {
    SCOPE.with_borrow(|s| {
        let mut out = String::with_capacity(text.len());
        let mut missing = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find(REF) {
            out.push_str(&rest[..at]);
            let token = rest.get(at..at + REF_LEN).unwrap_or(&rest[at..]);
            let target = u64::from_str_radix(&token[REF.len()..], 16)
                .ok()
                .and_then(|id| s.refs.get(&id))
                .map(|&(target, name)| (s.targets.get(&target), name));
            match target {
                Some((Some(t), _)) => {
                    out.push_str(prefix);
                    out.push_str(t.file);
                    if let Some(anchor) = &t.anchor {
                        out.push('#');
                        out.push_str(anchor);
                    }
                }
                Some((None, name)) => {
                    missing.push(name);
                    out.push('#');
                }
                // not one of ours: keep as written
                None => out.push_str(token),
            }
            rest = &rest[at + token.len()..];
        }
        out.push_str(rest);
        if missing.is_empty() {
            Ok(out)
        } else {
            missing.sort_unstable();
            missing.dedup();
            Err(missing)
        }
    })
}

fn ref_id(id: Ref) -> u64 {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

/// The id mdBook gives the first heading in `markdown`: its text, lowercased, spaces as `-`,
/// other punctuation dropped. `None` without a heading (links then go to the page).
fn heading_id(markdown: &str) -> Option<String> {
    let line = markdown
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with('#'))?;
    let text = line.trim_start_matches('#');
    // the text, without inline HTML (`<ins>`) and markdown marks
    let mut plain = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '`' | '*' => {}
            c if !in_tag => plain.push(c),
            _ => {}
        }
    }
    // mdBook's `normalize_id`
    let id: String = plain
        .trim()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    (!id.is_empty()).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OnPage;
    struct Embedded;
    struct Nowhere;

    #[test]
    fn links_resolve_after_rendering() {
        reset();
        let missing = render(link_to::<Nowhere>("c"));
        register_page(TypeId::of::<OnPage>(), "page.md");
        in_page("other.md", || {
            register_here::<Embedded>(|| "intro\n\n## Number Input\n\ntext".into());
            // already registered: not rendered again
            register_here::<Embedded>(|| unreachable!());
        });
        // outside the book: nothing to register, nothing rendered
        register_here::<Nowhere>(|| unreachable!());

        assert_eq!(resolve(&missing, "").unwrap_err(), [type_name::<Nowhere>()]);

        in_page("cmds.md", || {
            register_name_here("ask.new_chat", "### `ask.new_chat`")
        });
        let named = render(link_to_name("ask.new_chat", "new chat"));
        assert_eq!(
            resolve(&named, "").unwrap(),
            "[new chat](cmds.md#asknew_chat)"
        );
        let typo = render(link_to_name("ask.new_caht", "x"));
        assert_eq!(resolve(&typo, "").unwrap_err(), ["ask.new_caht"]);
        let ok = format!(
            "{} {}",
            render(link_to::<OnPage>("a")),
            render(link_to::<Embedded>("b"))
        );
        assert_eq!(
            resolve(&ok, "docs/src/").unwrap(),
            "[a](docs/src/page.md) [b](docs/src/other.md#number-input)"
        );
    }

    #[test]
    fn heading_ids_match_mdbook() {
        assert_eq!(
            heading_id("## Number Input").as_deref(),
            Some("number-input")
        );
        assert_eq!(
            heading_id("x\n## <ins>Build</ins> from `source`!").as_deref(),
            Some("build-from-source")
        );
        assert_eq!(heading_id("no heading"), None);
    }

    fn render(span: Span) -> String {
        let mut out = String::new();
        let _ = span.render(&mut out);
        out
    }
}
