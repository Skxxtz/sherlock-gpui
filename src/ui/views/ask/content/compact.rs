/// Lines longer than this (in chars) are cut.
const MAX_LINE: usize = 400;
/// Older output up to this many lines per block is sent whole.
const OLD_OUTPUT_MAX: usize = 80;
/// Longer blocks keep this many lines at each end.
const OLD_OUTPUT_LINES: usize = 30;
const PROBLEM_WORDS: [&str; 9] = [
    "error",
    "fail",
    "warn",
    "denied",
    "not found",
    "panic",
    "fatal",
    "cannot",
    "unable",
];
/// At most this many problem lines from the middle, so a flood of warnings stays bounded.
const MAX_PROBLEM_LINES: usize = 40;

fn is_problem(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    PROBLEM_WORDS.iter().any(|w| lower.contains(w))
}

/// Command output, compacted line by line.
pub fn output(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.lines().map(str::trim_end).peekable();
    let mut blank = false;
    while let Some(line) = lines.next() {
        if line.is_empty() {
            if !blank && !out.is_empty() {
                out.push('\n');
            }
            blank = true;
            continue;
        }
        blank = false;
        let mut repeats = 1;
        while lines.peek() == Some(&line) {
            lines.next();
            repeats += 1;
        }
        push_cut(&mut out, line);
        if repeats > 1 {
            out.push_str(&format!("  (×{repeats})"));
        }
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

fn push_cut(out: &mut String, line: &str) {
    match line.char_indices().nth(MAX_LINE) {
        Some((at, _)) => {
            out.push_str(&line[..at]);
            out.push_str(" …");
        }
        None => out.push_str(line),
    }
}

/// An earlier answer as it's sent back: without its ```next block and extra blank lines.
pub fn answer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    let mut blank = 0;
    for line in text.lines() {
        let fence = line.trim_start();
        if skipping {
            skipping = !fence.starts_with("```");
            continue;
        }
        if fence
            .strip_prefix("```")
            .is_some_and(|l| l.trim().eq_ignore_ascii_case("next"))
        {
            skipping = true;
            continue;
        }
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.trim_end().to_owned()
}

/// Output attached to an older question: long fenced blocks keep both ends and problem lines.
pub fn old_attachment(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(4096));
    let mut block: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let fence = line.trim_start().starts_with("```");
        if inside && !fence {
            block.push(line);
            continue;
        }
        if inside {
            // Closing fence: the block, shortened.
            let n = block.len();
            if n > OLD_OUTPUT_MAX {
                for l in &block[..OLD_OUTPUT_LINES] {
                    out.push_str(l);
                    out.push('\n');
                }
                let middle = &block[OLD_OUTPUT_LINES..n - OLD_OUTPUT_LINES];
                let mut skipped = 0;
                let mut kept = 0;
                for l in middle {
                    if kept < MAX_PROBLEM_LINES && is_problem(l) {
                        if skipped > 0 {
                            out.push_str(&format!("… ({skipped} lines left out)\n"));
                            skipped = 0;
                        }
                        out.push_str(l);
                        out.push('\n');
                        kept += 1;
                    } else {
                        skipped += 1;
                    }
                }
                if skipped > 0 {
                    out.push_str(&format!("… ({skipped} lines left out)\n"));
                }
                for l in &block[n - OLD_OUTPUT_LINES..] {
                    out.push_str(l);
                    out.push('\n');
                }
            } else {
                for l in &block {
                    out.push_str(l);
                    out.push('\n');
                }
            }
            block.clear();
        }
        inside = fence && !inside;
        out.push_str(line);
        out.push('\n');
    }
    // An unclosed block stays as it was.
    for l in block {
        out.push_str(l);
        out.push('\n');
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_keeps_content_drops_noise() {
        let text = "a   \n\n\n\nb\nb\nb\nc\n\n";
        assert_eq!(output(text), "a\n\nb  (×3)\nc");
        let long = "x".repeat(MAX_LINE + 10);
        assert_eq!(output(&long).chars().count(), MAX_LINE + 2);
        assert_eq!(output(""), "");
    }

    #[test]
    fn answer_drops_suggestions() {
        let text = "Lead.\n\n\n```sh\nls\n```\n\n```next\nwhat now?\n```\n";
        assert_eq!(answer(text), "Lead.\n\n```sh\nls\n```");
        assert_eq!(answer("```NEXT\nq\n```\nafter"), "after");
    }

    #[test]
    fn old_attachments_keep_both_ends() {
        let mut body: Vec<String> = (0..100).map(|i| i.to_string()).collect();
        body[50] = "E: permission denied".into();
        let text = format!(
            "I ran:\n```sh\nls\n```\noutput:\n```text\n{}\n```",
            body.join("\n")
        );
        let short = old_attachment(&text);
        assert!(short.contains("```sh\nls\n```"), "short blocks stay whole");
        assert!(short.contains(
            "\n29\n… (20 lines left out)\nE: permission denied\n… (19 lines left out)\n70\n"
        ));
        assert!(short.ends_with("99\n```"));
        let medium: Vec<String> = (0..80).map(|i| i.to_string()).collect();
        let text = format!("```text\n{}\n```", medium.join("\n"));
        assert_eq!(old_attachment(&text), text, "up to 80 lines stay whole");
    }
}
