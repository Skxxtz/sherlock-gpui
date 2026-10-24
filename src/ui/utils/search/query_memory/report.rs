use std::fmt::{self, Write as _};

use super::{
    ALPHA, MIN_P,
    memory::{Node, QueryMemory, ROOT, normalize},
};

const BAR_WIDTH: usize = 20;
/// Two totals closer than this count as "the same evidence".
const EPS: f32 = 1e-3;

/// ANSI styling, switched off when stdout isn't a terminal or `NO_COLOR` is set.
#[derive(Clone, Copy)]
struct Style(bool);

impl Style {
    fn detect() -> Self {
        use std::io::IsTerminal;
        Self(std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
    }
    fn paint(self, code: &str, s: impl fmt::Display) -> String {
        if self.0 {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
    fn bold(self, s: impl fmt::Display) -> String {
        self.paint("1", s)
    }
    fn dim(self, s: impl fmt::Display) -> String {
        self.paint("2", s)
    }
    fn cyan(self, s: impl fmt::Display) -> String {
        self.paint("36", s)
    }
}

fn bar(p: f32) -> String {
    let filled = ((p * BAR_WIDTH as f32).ceil() as usize).min(BAR_WIDTH);
    format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled))
}

fn ago(days: u32) -> String {
    match days {
        0 => "today".into(),
        1 => "yesterday".into(),
        d if d < 60 => format!("{d}d ago"),
        d => format!("{}mo ago", d / 30),
    }
}

impl QueryMemory {
    /// Everything learned, as a compressed prefix tree.
    pub fn report(&self, today: u32) -> String {
        self.report_styled(today, Style::detect())
    }

    /// `P̂(item | query)` for one query and how it changes the ranking.
    pub fn explain(&self, query: &str, today: u32) -> String {
        self.explain_styled(query, today, Style::detect())
    }

    /// A prefix adds nothing when one child carries exactly the same picks
    /// ("f" when everything typed after it continued as "fi").
    fn is_redundant(&self, prefix: &str, node: &Node, today: u32) -> bool {
        let total = node.total.at(today);
        let child_len = prefix.chars().count() + 1;
        self.nodes
            .range::<str, _>((
                std::ops::Bound::Excluded(prefix),
                std::ops::Bound::Unbounded,
            ))
            .take_while(|(k, _)| k.starts_with(prefix))
            .filter(|(k, _)| k.chars().count() == child_len)
            .any(|(_, child)| (child.total.at(today) - total).abs() < EPS)
    }

    /// One line per item: probability bar, %, key, evidence.
    fn write_node(&self, out: &mut String, node: &Node, today: u32, st: Style) {
        let denom = node.total.at(today) + ALPHA;
        let mut rows: Vec<_> = node.hits.iter().map(|h| (h, h.count.at(today))).collect();
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));

        for (h, c) in rows {
            let p = c / denom;
            let _ = writeln!(
                out,
                "  {} {:>5.1}%  {}  {}",
                bar(p),
                p * 100.0,
                h.key,
                st.dim(format!(
                    "· {c:.1} picks · last {}",
                    ago(today.saturating_sub(h.count.day()))
                ))
            );
        }
    }

    fn report_styled(&self, today: u32, st: Style) -> String {
        if self.is_empty() {
            return "Nothing learned yet. Picks are recorded when you launch something after typing a query.\n".into();
        }

        let shown: Vec<_> = self
            .nodes
            .iter()
            .filter(|(q, n)| !self.is_redundant(q, n, today))
            .collect();

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{} {}\n",
            st.bold("Query memory"),
            st.dim(format!(
                "· {} prefixes ({} shown) · score × (1 − P(item | query))",
                self.len(),
                shown.len()
            ))
        );

        // BTreeMap order is prefix-tree order: "f", "fi", "fir…", "fl…"
        for (q, node) in shown {
            let label = if q == ROOT {
                "(any query)".to_owned()
            } else {
                format!("\"{q}\"")
            };
            let _ = writeln!(
                out,
                "{} {}",
                st.cyan(label),
                st.dim(format!("{:.1} picks", node.total.at(today)))
            );
            self.write_node(&mut out, node, today, st);
            out.push('\n');
        }
        out
    }

    fn explain_styled(&self, query: &str, today: u32, st: Style) -> String {
        let Some(q) = normalize(query) else {
            return "Empty query: nothing learned applies, ranking uses counts and priority only.\n".into();
        };

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{} {}\n",
            st.bold("P(item |"),
            st.cyan(format!("\"{q}\")"))
        );

        let Some(node) = self.nodes.get(&q) else {
            // longest prefix that *was* learned, to show where knowledge ends
            let learned = q
                .char_indices()
                .rev()
                .map(|(i, c)| &q[..i + c.len_utf8()])
                .find(|p| self.nodes.contains_key(*p));
            let _ = match learned {
                Some(p) => writeln!(
                    out,
                    "Nothing learned for \"{q}\" yet (longest learned prefix: \"{p}\"): ranking is unchanged."
                ),
                None => writeln!(out, "Nothing learned for \"{q}\": ranking is unchanged."),
            };
            return out;
        };

        let total = node.total.at(today);
        let _ = writeln!(
            out,
            "{}",
            st.dim(format!(
                "{total:.1} picks under this prefix · P = picks / ({total:.1} + {ALPHA})"
            ))
        );
        self.write_node(&mut out, node, today, st);

        let _ = writeln!(
            out,
            "\n{}",
            st.dim("effect on the match score (lower is better)")
        );
        let mut ps: Vec<_> = self.probabilities(&q, today).into_iter().collect();
        ps.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (key, p) in ps {
            let effect = if p < MIN_P {
                st.dim("ignored (below threshold)")
            } else {
                format!("× {:.2}", 1.0 - p)
            };
            let _ = writeln!(out, "  {effect:<8}  {key}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::item_key::ItemKey;

    fn key(s: &str) -> ItemKey {
        ItemKey::new("app", s)
    }

    #[test]
    fn report_collapses_single_paths() {
        let mut m = QueryMemory::default();
        m.record("firefox", &key("firefox.desktop"), 10);
        m.record("files", &key("files.desktop"), 10);
        let r = m.report_styled(12, Style(false));
        // "fi" is where the two picks split, so it's shown; "f" holds exactly
        // the same evidence as "fi" and is collapsed into it
        assert!(r.contains("\"fi\""), "{r}");
        assert!(!r.contains("\"f\""), "{r}");
        // the chain "fir" → "firefox" carries one item: only the leaf is shown
        assert!(!r.contains("\"fir\""), "{r}");
        assert!(r.contains("\"firefox\""), "{r}");
        assert!(r.contains("2d ago"));
    }

    #[test]
    fn report_empty() {
        assert!(
            QueryMemory::default()
                .report_styled(0, Style(false))
                .starts_with("Nothing learned")
        );
    }

    #[test]
    fn explain_matches_probabilities() {
        let mut m = QueryMemory::default();
        for _ in 0..3 {
            m.record("firefox", &key("firefox.desktop"), 0);
        }
        m.record("files", &key("files.desktop"), 0);
        let e = m.explain_styled("fi", 0, Style(false));
        for item in ["firefox.desktop", "files.desktop"] {
            let p = m.boosts("fi", 0).get(&key(item));
            assert!(e.contains(&format!("× {:.2}", 1.0 - p)), "{e}");
            assert!(e.contains(&format!("app:{item}")), "{e}");
        }
        assert!(e.contains("60.0%"), "{e}"); // 3 / (4 + 1)
    }

    #[test]
    fn explain_unknown_shows_where_knowledge_ends() {
        let mut m = QueryMemory::default();
        m.record("fire", &key("firefox.desktop"), 0);
        let e = m.explain_styled("firefox", 0, Style(false));
        assert!(e.contains("longest learned prefix: \"fire\""), "{e}");
        assert!(
            m.explain_styled("zz", 0, Style(false))
                .contains("ranking is unchanged")
        );
    }
}
