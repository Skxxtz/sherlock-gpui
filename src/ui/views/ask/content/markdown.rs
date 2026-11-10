use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inline {
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    /// The streaming cursor the view appends to the last text.
    Caret,
}

/// Text without markup, plus the styled byte ranges in it (possibly overlapping).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rich {
    pub text: String,
    pub spans: Vec<(Range<usize>, Inline)>,
}

impl Rich {
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Splits at byte `at` (a char boundary), cutting spans that cross it.
    pub fn split_at(self, at: usize) -> (Rich, Rich) {
        let (a, b) = self.text.split_at(at);
        let mut left = Rich {
            text: a.to_owned(),
            spans: Vec::new(),
        };
        let mut right = Rich {
            text: b.trim_start().to_owned(),
            spans: Vec::new(),
        };
        let shift = at + (b.len() - right.text.len());
        for (r, kind) in self.spans {
            if r.start < at {
                left.spans.push((r.start..r.end.min(at), kind));
            }
            if r.end > shift {
                right
                    .spans
                    .push((r.start.max(shift) - shift..r.end - shift, kind));
            }
        }
        (left, right)
    }

    /// End of the first sentence: after `.`/`?`/`!`/`:` followed by whitespace, outside code.
    pub fn first_sentence_end(&self) -> Option<usize> {
        let in_code = |i: usize| {
            self.spans
                .iter()
                .any(|(r, k)| *k == Inline::Code && r.start <= i && i < r.end)
        };
        let bytes = self.text.as_bytes();
        (0..bytes.len().saturating_sub(1)).find_map(|i| {
            (matches!(bytes[i], b'.' | b'?' | b'!' | b':')
                && bytes[i + 1].is_ascii_whitespace()
                && !in_code(i))
            .then_some(i + 1)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Paragraph(Rich),
    Heading(u8, Rich),
    /// `start`: the first number of an ordered list, `None` for bullets.
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    /// `lang` is the fence info's first word ("" if none). `terminal`: the info also says
    /// `terminal` (```` ```sh terminal ````), the model's mark for commands that need a real
    /// terminal. `closed` is false while streaming.
    Code {
        lang: String,
        body: String,
        closed: bool,
        terminal: bool,
    },
    Quote(Vec<Block>),
    Table {
        head: Vec<Rich>,
        rows: Vec<Vec<Rich>>,
    },
    Rule,
}

/// Appends the streaming cursor `▍` to the last text in the document (not into code).
pub fn append_caret(blocks: &mut [Block]) {
    fn last_rich(blocks: &mut [Block]) -> Option<&mut Rich> {
        match blocks.last_mut()? {
            Block::Paragraph(r) | Block::Heading(_, r) => Some(r),
            Block::List { items, .. } => items.last_mut().and_then(|i| last_rich(i)),
            Block::Quote(inner) => last_rich(inner),
            _ => None,
        }
    }
    if let Some(rich) = last_rich(blocks) {
        let start = rich.text.len();
        rich.text.push('▍');
        rich.spans.push((start..rich.text.len(), Inline::Caret));
    }
}

impl Block {
    /// Whether this is a code block a shell can run.
    pub fn is_runnable(&self) -> bool {
        match self {
            Block::Code {
                lang, body, closed, ..
            } => *closed && !body.trim().is_empty() && is_shell(lang),
            _ => false,
        }
    }
}

/// Languages a code block may be run as.
pub fn is_shell(lang: &str) -> bool {
    matches!(
        lang.to_ascii_lowercase().as_str(),
        "" | "sh" | "shell" | "bash" | "zsh" | "fish" | "console" | "terminal"
    )
}

/// Every code block in document order, including ones nested in lists and quotes.
pub fn code_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut out = Vec::new();
    fn walk<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
        for b in blocks {
            match b {
                Block::Code { .. } => out.push(b),
                Block::List { items, .. } => items.iter().for_each(|i| walk(i, out)),
                Block::Quote(inner) => walk(inner, out),
                _ => {}
            }
        }
    }
    walk(blocks, &mut out);
    out
}

/// The script of a runnable block: `$ ` prompts from console transcripts are dropped, and
/// output lines (those without a prompt) too, when the block has prompts at all.
pub fn script(body: &str) -> String {
    let has_prompts = body.lines().any(|l| l.trim_start().starts_with("$ "));
    if !has_prompts {
        return body.trim().to_owned();
    }
    body.lines()
        .filter_map(|l| l.trim_start().strip_prefix("$ "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A container being filled.
enum Frame {
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Item(Vec<Block>),
    Table {
        head: Vec<Rich>,
        rows: Vec<Vec<Rich>>,
        row: Vec<Rich>,
        in_head: bool,
    },
}

/// What inline text currently flows into.
enum Leaf {
    Paragraph,
    Heading(u8),
    Cell,
    /// Text inside a tight list item, with no explicit paragraph.
    Implicit,
}

struct Builder {
    root: Vec<Block>,
    stack: Vec<Frame>,
    leaf: Option<(Leaf, Rich)>,
    /// Open inline styles: kind and start offset.
    styles: Vec<(Inline, usize)>,
    code: Option<(String, bool, String)>,
}

pub fn parse(text: &str) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut b = Builder {
        root: Vec::new(),
        stack: Vec::new(),
        leaf: None,
        styles: Vec::new(),
        code: None,
    };
    for event in Parser::new_ext(text, opts) {
        b.event(event);
    }
    // A fence still open at the end of the text (streaming): pulldown already closed it, but
    // it must not count as runnable yet.
    b.flush_leaf();
    if ends_in_open_fence(text) {
        mark_last_code_open(&mut b.root);
    }
    b.root
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => {
                if let Some((_, _, body)) = &mut self.code {
                    body.push_str(&t);
                } else {
                    self.text(&t);
                }
            }
            Event::Code(t) => {
                let rich = self.rich();
                let start = rich.text.len();
                rich.text.push_str(&t);
                let end = rich.text.len();
                rich.spans.push((start..end, Inline::Code));
            }
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.text("\n"),
            Event::Html(t) | Event::InlineHtml(t) => self.text(&t),
            Event::TaskListMarker(done) => self.text(if done { "☑ " } else { "☐ " }),
            Event::Rule => {
                self.flush_leaf();
                self.push(Block::Rule);
            }
            Event::FootnoteReference(t) => self.text(&format!("[{t}]")),
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.open_leaf(Leaf::Paragraph),
            Tag::Heading { level, .. } => self.open_leaf(Leaf::Heading(heading_level(level))),
            Tag::BlockQuote(_) => {
                self.flush_leaf();
                self.stack.push(Frame::Quote(Vec::new()));
            }
            Tag::CodeBlock(kind) => {
                self.flush_leaf();
                let (lang, terminal) = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let mut words = info.split_whitespace();
                        let lang = words.next().unwrap_or("").to_owned();
                        (lang, words.any(|w| w.eq_ignore_ascii_case("terminal")))
                    }
                    CodeBlockKind::Indented => (String::new(), false),
                };
                self.code = Some((lang, terminal, String::new()));
            }
            Tag::List(start) => {
                self.flush_leaf();
                self.stack.push(Frame::List {
                    start,
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                self.flush_leaf();
                self.stack.push(Frame::Item(Vec::new()));
            }
            Tag::Table(_) => {
                self.flush_leaf();
                self.stack.push(Frame::Table {
                    head: Vec::new(),
                    rows: Vec::new(),
                    row: Vec::new(),
                    in_head: false,
                });
            }
            Tag::TableHead => {
                if let Some(Frame::Table { in_head, .. }) = self.stack.last_mut() {
                    *in_head = true;
                }
            }
            Tag::TableCell => self.open_leaf(Leaf::Cell),
            Tag::Emphasis => self.open_style(Inline::Italic),
            Tag::Strong => self.open_style(Inline::Bold),
            Tag::Strikethrough => self.open_style(Inline::Strike),
            Tag::Link { .. } => self.open_style(Inline::Link),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.flush_leaf(),
            TagEnd::TableCell => {
                let cell = self.leaf.take().map(|(_, r)| r).unwrap_or_default();
                self.styles.clear();
                if let Some(Frame::Table { row, .. }) = self.stack.last_mut() {
                    row.push(cell);
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(Frame::Table {
                    head,
                    rows,
                    row,
                    in_head,
                }) = self.stack.last_mut()
                {
                    let cells = std::mem::take(row);
                    if *in_head {
                        *head = cells;
                        *in_head = false;
                    } else {
                        rows.push(cells);
                    }
                }
            }
            TagEnd::CodeBlock => {
                if let Some((lang, terminal, body)) = self.code.take() {
                    let body = body.trim_end_matches('\n').to_owned();
                    self.push(Block::Code {
                        lang,
                        body,
                        closed: true,
                        terminal,
                    });
                }
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::Table => {
                self.flush_leaf();
                let block = match self.stack.pop() {
                    Some(Frame::Quote(inner)) => Block::Quote(inner),
                    Some(Frame::List { start, items }) => Block::List { start, items },
                    Some(Frame::Item(inner)) => {
                        if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                            items.push(inner);
                        }
                        return;
                    }
                    Some(Frame::Table { head, rows, .. }) => Block::Table { head, rows },
                    None => return,
                };
                self.push(block);
            }
            TagEnd::Emphasis => self.close_style(Inline::Italic),
            TagEnd::Strong => self.close_style(Inline::Bold),
            TagEnd::Strikethrough => self.close_style(Inline::Strike),
            TagEnd::Link => self.close_style(Inline::Link),
            _ => {}
        }
    }

    fn open_leaf(&mut self, leaf: Leaf) {
        self.flush_leaf();
        self.leaf = Some((leaf, Rich::default()));
    }

    /// The inline buffer; tight list items get an implicit paragraph.
    fn rich(&mut self) -> &mut Rich {
        if self.leaf.is_none() {
            self.leaf = Some((Leaf::Implicit, Rich::default()));
        }
        &mut self.leaf.as_mut().unwrap().1
    }

    fn text(&mut self, t: &str) {
        self.rich().text.push_str(t);
    }

    fn open_style(&mut self, kind: Inline) {
        let at = self.rich().text.len();
        self.styles.push((kind, at));
    }

    fn close_style(&mut self, kind: Inline) {
        if let Some(i) = self.styles.iter().rposition(|(k, _)| *k == kind) {
            let (_, start) = self.styles.remove(i);
            let rich = self.rich();
            let end = rich.text.len();
            if start < end {
                rich.spans.push((start..end, kind));
            }
        }
    }

    fn flush_leaf(&mut self) {
        let Some((leaf, rich)) = self.leaf.take() else {
            return;
        };
        self.styles.clear();
        if rich.is_empty() {
            return;
        }
        let block = match leaf {
            Leaf::Heading(level) => Block::Heading(level, rich),
            Leaf::Paragraph | Leaf::Implicit => Block::Paragraph(rich),
            // A cell outside a table row (malformed input): keep the text.
            Leaf::Cell => Block::Paragraph(rich),
        };
        self.push(block);
    }

    fn push(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Quote(v) | Frame::Item(v)) => v.push(block),
            _ => self.root.push(block),
        }
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Whether the text ends inside a fenced code block.
fn ends_in_open_fence(text: &str) -> bool {
    let mut open: Option<(u8, usize)> = None;
    for line in text.lines() {
        let l = line.trim_start();
        let Some(ch) = l.bytes().next().filter(|c| *c == b'`' || *c == b'~') else {
            continue;
        };
        let n = l.bytes().take_while(|c| *c == ch).count();
        if n < 3 {
            continue;
        }
        open = match open {
            None => Some((ch, n)),
            Some((c, m)) if c == ch && n >= m && l[n..].trim().is_empty() => None,
            other => other,
        };
    }
    open.is_some()
}

fn mark_last_code_open(blocks: &mut [Block]) {
    fn last_code(blocks: &mut [Block]) -> Option<&mut Block> {
        let last = blocks.last_mut()?;
        match last {
            Block::Code { .. } => Some(last),
            Block::List { items, .. } => items.last_mut().and_then(|i| last_code(i)),
            Block::Quote(inner) => last_code(inner),
            _ => None,
        }
    }
    if let Some(Block::Code { closed, .. }) = last_code(blocks) {
        *closed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styled<'a>(r: &'a Rich, kind: Inline) -> Vec<&'a str> {
        r.spans
            .iter()
            .filter(|(_, k)| *k == kind)
            .map(|(rg, _)| &r.text[rg.clone()])
            .collect()
    }

    fn para(b: &Block) -> &Rich {
        match b {
            Block::Paragraph(r) => r,
            other => panic!("not a paragraph: {other:?}"),
        }
    }

    #[test]
    fn inline_markup() {
        let b = parse("Use **ss** with `-ltnp`, *care* and ~~not~~ [docs](https://x.y).");
        let r = para(&b[0]);
        assert_eq!(r.text, "Use ss with -ltnp, care and not docs.");
        assert_eq!(styled(r, Inline::Bold), ["ss"]);
        assert_eq!(styled(r, Inline::Code), ["-ltnp"]);
        assert_eq!(styled(r, Inline::Italic), ["care"]);
        assert_eq!(styled(r, Inline::Strike), ["not"]);
        assert_eq!(styled(r, Inline::Link), ["docs"]);
    }

    #[test]
    fn multiple_multiline_code_blocks() {
        let text = "Install:\n\n```bash\nsudo apt update\nsudo apt install ripgrep\n```\n\nConfig:\n\n```toml\n[a]\nb = 1\n```\n";
        let b = parse(text);
        let codes = code_blocks(&b);
        assert_eq!(codes.len(), 2);
        assert_eq!(
            codes[0],
            &Block::Code {
                lang: "bash".into(),
                body: "sudo apt update\nsudo apt install ripgrep".into(),
                closed: true,
                terminal: false,
            }
        );
        assert!(codes[0].is_runnable());
        assert!(!codes[1].is_runnable(), "toml isn't a shell");
    }

    #[test]
    fn streaming_fence_is_open() {
        let b = parse("Run:\n\n```sh\nsudo ss -l");
        let codes = code_blocks(&b);
        assert!(matches!(codes[0], Block::Code { closed: false, .. }));
        assert!(!codes[0].is_runnable());
    }

    #[test]
    fn nested_lists_and_tight_items() {
        let b = parse("1. first\n2. second\n   - inner **bold**\n3. third\n");
        let Block::List { start, items } = &b[0] else {
            panic!()
        };
        assert_eq!(*start, Some(1));
        assert_eq!(items.len(), 3);
        assert_eq!(para(&items[0][0]).text, "first");
        let Block::List {
            start: None,
            items: inner,
        } = &items[1][1]
        else {
            panic!("{:?}", items[1])
        };
        assert_eq!(styled(para(&inner[0][0]), Inline::Bold), ["bold"]);
    }

    #[test]
    fn code_inside_list_item_is_found() {
        let b = parse("- Step one:\n\n  ```sh\n  ls -la\n  ```\n");
        let codes = code_blocks(&b);
        assert_eq!(codes.len(), 1);
        assert!(codes[0].is_runnable());
    }

    #[test]
    fn tables_quotes_rules_headings() {
        let b = parse("# Title\n\n> quoted\n\n---\n\n| a | b |\n|---|---|\n| 1 | **2** |\n");
        assert!(matches!(&b[0], Block::Heading(1, r) if r.text == "Title"));
        assert!(matches!(&b[1], Block::Quote(inner) if para(&inner[0]).text == "quoted"));
        assert_eq!(b[2], Block::Rule);
        let Block::Table { head, rows } = &b[3] else {
            panic!()
        };
        assert_eq!(
            head.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(styled(&rows[0][1], Inline::Bold), ["2"]);
    }

    #[test]
    fn unclosed_emphasis_stays_literal() {
        let b = parse("This is **impor");
        assert_eq!(para(&b[0]).text, "This is **impor");
    }

    #[test]
    fn first_sentence_skips_code() {
        let r = para(&parse("Open `a.b c` now. Then go.")[0]).clone();
        let at = r.first_sentence_end().unwrap();
        let (lead, rest) = r.split_at(at);
        assert_eq!(lead.text, "Open a.b c now.");
        assert_eq!(styled(&lead, Inline::Code), ["a.b c"]);
        assert_eq!(rest.text, "Then go.");
    }

    #[test]
    fn console_transcripts() {
        assert_eq!(script("$ ls\nfile.txt\n$ pwd\n/home"), "ls\npwd");
        assert_eq!(script("ls -la\n"), "ls -la");
    }
}
