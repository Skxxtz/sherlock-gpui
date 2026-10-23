use std::cmp::Ordering;

use crate::loader::utils::Priority;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct SortKey {
    base: u16,
    score: u16,
    count: u16,
    tie: u16,
}
impl Priority {
    pub fn sort_key(self, query: &str, match_in: &str, p: f32) -> SortKey {
        let text = search_score(query, match_in);
        SortKey {
            base: self.base,
            score: if text < 10_000 {
                (text as f32 * (1.0 - p)).round() as u16
            } else {
                text
            },
            tie: self.tie,
            count: self.count,
        }
    }
}
impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base
            .cmp(&other.base)
            .then(self.score.cmp(&other.score))
            .then(other.count.cmp(&self.count))
            .then(self.tie.cmp(&other.tie))
    }
}
impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Scores how well `query` matches an item's search text. **Lower is better.**
///
/// `match_in` is the item's search string as built by `construct_search`:
/// the display name and keywords, separated by `;` (e.g. `"visual studio code;code;editor"`).
/// Every element is scored separately and the best score wins, so a keyword
/// match is as good as a name match.
///
/// # Score bands
///
/// Each band sits below the next, so a stronger kind of match always ranks
/// above a weaker one:
///
/// | Score         | Match                                   | `query = "calc"`        |
/// |---------------|-----------------------------------------|-------------------------|
/// | `0`           | exact, same case                        | `calc`                  |
/// | `100`         | exact, ignoring case                    | `CALC`                  |
/// | `1000..=2000` | element starts with the query           | `calculator` → 1600     |
/// | `2500..=4500` | query occurs inside the element         | `my_calc_app` → 3409    |
/// | `3500..=10000`| similar spelling (Levenshtein)          | `clac` → 5000           |
/// | `8000`        | empty query (home screen)               | —                       |
/// | `10000`       | no match / empty search text            | `firefox`               |
///
/// # How each band is computed
///
/// - **Prefix:** `1000 + 1000 · (1 − coverage)`, where `coverage` is the share
///   of the element the query covers. Typing more, or matching a shorter name,
///   ranks higher.
/// - **Substring:** `2500 + 1000 · (1 − coverage) + position penalty`, where the
///   penalty (`0..=1000`) grows the further into the element the match starts.
/// - **Fuzzy:** only tried when the lengths differ by less than
///   `query.len() / 4 + 1` (at most 4). The edit distance is normalized by the
///   element length, clamped to `0.35..=1.0` and scaled to `0..=10000`.
///
/// Exact matches return immediately, since nothing can beat them.
///
/// # Notes
///
/// - Lengths are measured in bytes, so coverage is approximate for non-ASCII
///   text (`ä`, `ß`, …).
/// - This is only the text score. The final ranking also uses the launcher's
///   `base` priority and the launch `count` (see [`SortKey`]), and learned
///   picks from the query memory are subtracted from this score.
fn search_score(query: &str, match_in: &str) -> u16 {
    if match_in.is_empty() {
        return 10_000u16;
    }
    if query.is_empty() {
        return 8_000u16;
    }

    let query_lower = query.to_lowercase();
    let mut best_score = 10_000u16;

    for element in match_in.split(';') {
        if element.is_empty() {
            continue;
        }

        // perfect match
        if element == query {
            return 0u16;
        }

        let element_lower = element.to_lowercase();

        // case-insensitive perfect match
        if element_lower == query_lower {
            return 100u16;
        }

        // prefix match
        if element_lower.starts_with(&query_lower) {
            let coverage = query.len() as f32 / element.len() as f32;
            let score = (1000.0 + 1000.0 * (1.0 - coverage)).round() as u16;
            best_score = best_score.min(score);
            continue;
        }

        // substring match (with position + coverage penalty)
        if let Some(pos) = element_lower.find(&query_lower) {
            let coverage = query.len() as f32 / element.len() as f32;
            let position_penalty = pos as f32 / element.len() as f32 * 1000.0;
            let score = (2500.0 + 1000.0 * (1.0 - coverage) + position_penalty).round() as u16;
            best_score = best_score.min(score);
            continue;
        }

        // levenshtein — window scales with query length
        let max_dist = (query.len() / 4 + 1).min(4);
        if (element.len() as isize - query.len() as isize).abs() < max_dist as isize {
            let dist = levenshtein::levenshtein(&query_lower, &element_lower);
            let normed = (dist as f32 / element.len() as f32).clamp(0.35, 1.0);
            let score = (normed * 10000.0).round() as u16;
            best_score = best_score.min(score);
        }
    }

    best_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_relevance_ranking() {
        let query = "calc";

        // Lower score is better in search_score logic
        let perfect = search_score(query, "calc");
        let case_insensitive = search_score(query, "CALC");
        let prefix = search_score(query, "calculator");
        let substring = search_score(query, "my_calc_app");
        let fuzzy = search_score(query, "clac"); // Levenshtein 1
        let no_match = search_score(query, "firefox");

        assert!(
            perfect < case_insensitive,
            "Perfect should beat case-insensitive"
        );
        assert!(
            case_insensitive < prefix,
            "Case-insensitive should beat prefix"
        );
        assert!(prefix < substring, "Prefix should beat substring");
        assert!(substring < fuzzy, "Substring should beat fuzzy/Levenshtein");
        assert!(fuzzy < no_match, "Fuzzy match should beat no match");
    }

    #[test]
    fn test_semicolon_alias_support() {
        let query = "code";
        let score = search_score(query, "Visual Studio;code;editor");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_levenshtein_scaling() {
        // Query "vlc" (length 3). max_dist = (3/4 + 1) = 1.
        // "vlc" vs "vlb" is distance 1. Should match.
        let score_match = search_score("vlc", "vlb");
        assert!(score_match < 10_000);

        // "vlc" vs "vxxxx" is length diff 2. Should be ignored by Levenshtein.
        let score_no_match = search_score("vlc", "vxxxx");
        assert_eq!(score_no_match, 10_000);
    }
}
