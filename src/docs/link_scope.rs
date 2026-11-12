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
    /// Where each type is documented.
    targets: HashMap<TypeId, Target>,
    /// Placeholder id → type, to resolve and to name it in errors.
    refs: HashMap<u64, (TypeId, &'static str)>,
}

struct Target {
    file: &'static str,
    anchor: Option<String>,
}

/// Markdown link to where `T` is documented. Resolved when the book is written.
pub fn link_to<T: 'static>(text: impl Into<std::borrow::Cow<'static, str>>) -> Span {
    let id = ref_id(TypeId::of::<T>());
    SCOPE.with_borrow_mut(|s| s.refs.insert(id, (TypeId::of::<T>(), type_name::<T>())));
    link(text, format!("{REF}{id:016x}"))
}

/// Starts a fresh render. Forgets every page, anchor and link.
pub(super) fn reset() {
    SCOPE.with_borrow_mut(|s| *s = Scope::default());
}

/// `id` is the page `file` itself. Wins over sections that embed the same type.
pub(super) fn register_page(id: TypeId, file: &'static str) {
    SCOPE.with_borrow_mut(|s| {
        s.targets.insert(id, Target { file, anchor: None });
    });
}

/// Renders with `file` as the current page.
pub(super) fn in_page<R>(file: &'static str, render: impl FnOnce() -> R) -> R {
    let outer = SCOPE.with_borrow_mut(|s| s.page.replace(file));
    let out = render();
    SCOPE.with_borrow_mut(|s| s.page = outer);
    out
}

/// Registers `T` as documented on the current page, at the first heading of its `markdown`. Does
/// nothing outside the book, or if `T` is registered already.
pub(super) fn register_here<T: 'static>(markdown: impl FnOnce() -> String) {
    let id = TypeId::of::<T>();
    let Some(page) = SCOPE.with_borrow(|s| s.page.filter(|_| !s.targets.contains_key(&id))) else {
        return;
    };
    // rendering may register nested sections: no borrow held meanwhile
    let anchor = heading_id(&markdown());
    SCOPE.with_borrow_mut(|s| {
        s.targets.entry(id).or_insert(Target { file: page, anchor });
    });
}

/// Replaces every placeholder in `text` with its target, `prefix` before the file. `Err`: the types
/// linked to but documented nowhere.
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
                .map(|&(ty, name)| (s.targets.get(&ty), name));
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

fn ref_id(id: TypeId) -> u64 {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

/// The id mdBook gives the first heading in `markdown`: its text, lowercased, spaces as `-`,
/// other punctuation dropped. `None` without a heading.
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
