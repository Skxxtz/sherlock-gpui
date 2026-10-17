use std::{io::Read, path::Path};

/// Limits for a text preview.
#[derive(Clone, Copy)]
pub struct TextPreview {
    /// Bytes read from the start of the file.
    pub max_bytes: usize,
    /// Lines shown.
    pub max_lines: usize,
}

impl Default for TextPreview {
    fn default() -> Self {
        Self {
            max_bytes: 4096,
            max_lines: 20,
        }
    }
}

impl TextPreview {
    /// First lines of the file, or `None` if it looks binary, isn't UTF-8, or is blank.
    pub fn read(self, path: impl AsRef<Path>) -> Option<String> {
        let mut buf = Vec::with_capacity(self.max_bytes);
        std::fs::File::open(path)
            .ok()?
            .take(self.max_bytes as u64)
            .read_to_end(&mut buf)
            .ok()?;

        if buf.contains(&0) {
            return None; // NUL byte → binary
        }
        // Cut off a UTF-8 character that may have been split at the byte limit.
        let text = match std::str::from_utf8(&buf) {
            Ok(s) => s,
            Err(e) if e.error_len().is_none() => {
                std::str::from_utf8(&buf[..e.valid_up_to()]).ok()?
            }
            Err(_) => return None, // not UTF-8
        };

        let lines: Vec<String> = text
            .lines()
            .take(self.max_lines)
            .map(|l| l.replace('\t', "    ").trim_end().to_owned())
            .collect();
        (!lines.iter().all(|l| l.is_empty())).then(|| lines.join("\n"))
    }
}
