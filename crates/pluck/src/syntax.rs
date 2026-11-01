#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSegment<'a> {
    Key(&'a str),
    Index(usize),
    Each,
}

/// Split `a.*.0.b` into segments. The empty path means "the value itself".
pub fn parse(raw: &str) -> Result<Vec<RawSegment<'_>>, &'static str> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    raw.split('.')
        .map(|s| match s {
            "" => Err("empty segment"),
            "*" => Ok(RawSegment::Each),
            s if s.bytes().all(|b| b.is_ascii_digit()) => s
                .parse()
                .map(RawSegment::Index)
                .map_err(|_| "index too large"),
            s => Ok(RawSegment::Key(s)),
        })
        .collect()
}
