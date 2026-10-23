use crate::{
    ui::query_memory::{MAX_HITS, MAX_QUERIES, memory::QueryMemory},
    utils::item_key::ItemKey,
};

fn key(s: &str) -> ItemKey {
    ItemKey::new("app", s)
}

fn p(m: &QueryMemory, q: &str, item: &str) -> f32 {
    m.probabilities(q, 0)
        .into_iter()
        .find(|(k, _)| k == &key(item))
        .map_or(0.0, |(_, p)| p)
}

#[track_caller]
fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn single_pick_is_half() {
    let mut m = QueryMemory::default();
    m.record("fierfox", &key("firefox"), 0);
    let b = m.boosts("fierfox", 0);
    approx(b.get(&key("firefox")), 0.5);
    approx(b.get(&key("chromium")), 0.0);
}

#[test]
fn every_prefix_learns() {
    let mut m = QueryMemory::default();
    m.record("firefox", &key("firefox"), 0);
    for q in ["f", "fi", "fir", "fire", "firef", "firefo", "firefox"] {
        approx(p(&m, q, "firefox"), 0.5);
    }
    approx(p(&m, "fx", "firefox"), 0.0);
    approx(p(&m, "firefoxx", "firefox"), 0.0); // typed beyond anything seen
}

#[test]
fn normalizes_case_and_whitespace() {
    let mut m = QueryMemory::default();
    m.record("  FireFox ", &key("firefox"), 0);
    approx(p(&m, "firefox", "firefox"), 0.5);
    approx(p(&m, "FIRE", "firefox"), 0.5);
}

#[test]
fn single_letter_is_share_of_picks_not_name_length() {
    let mut m = QueryMemory::default();
    for _ in 0..10 {
        m.record("flowchart", &key("flowchart"), 0);
    }
    for _ in 0..3 {
        m.record("firefox", &key("firefox"), 0);
    }
    m.record("files", &key("files"), 0);

    // c(f) = 14, α = 1 → 10/15, 3/15, 1/15
    approx(p(&m, "f", "flowchart"), 10.0 / 15.0);
    approx(p(&m, "f", "firefox"), 3.0 / 15.0);
    approx(p(&m, "f", "files"), 1.0 / 15.0);

    // "fi" no longer contains flowchart: c(fi) = 4 → 3/5, 1/5
    approx(p(&m, "fi", "flowchart"), 0.0);
    approx(p(&m, "fi", "firefox"), 3.0 / 5.0);
    approx(p(&m, "fi", "files"), 1.0 / 5.0);
}

#[test]
fn probabilities_sum_below_one() {
    let mut m = QueryMemory::default();
    for (q, i) in [("term", "kitty"), ("term", "alacritty"), ("te", "telegram")] {
        m.record(q, &key(i), 0);
    }
    let sum: f32 = m.probabilities("te", 0).iter().map(|(_, p)| p).sum();
    // α keeps mass for "something else": 3 / (3 + 1)
    approx(sum, 0.75);
}

#[test]
fn more_picks_more_confident_but_never_certain() {
    let mut m = QueryMemory::default();
    m.record("term", &key("kitty"), 0);
    let one = m.boosts("term", 0).get(&key("kitty"));
    for _ in 0..1000 {
        m.record("term", &key("kitty"), 0);
    }
    let many = m.boosts("term", 0).get(&key("kitty"));
    // grows with evidence, but stays < 1: `score · (1 − p)` never reaches an exact match's 0
    assert!(many > one && many < 1.0, "{one} → {many}");
}

#[test]
fn picks_decay_with_half_life() {
    let mut m = QueryMemory::default();
    for _ in 0..4 {
        m.record("term", &key("kitty"), 0);
    }
    // weights and total decay together, so the share stays the same …
    approx(m.probabilities("term", 0)[0].1, 4.0 / 5.0);
    // … but the evidence shrinks, so confidence drops: 2 / (2 + 1) after one half-life
    approx(m.probabilities("term", 30)[0].1, 2.0 / 3.0);
}

#[test]
fn old_picks_decay_and_new_habit_wins() {
    let mut m = QueryMemory::default();
    for _ in 0..5 {
        m.record("term", &key("kitty"), 0);
    }
    for _ in 0..3 {
        m.record("term", &key("alacritty"), 120);
    }
    let b = m.boosts("term", 120);
    assert!(b.get(&key("alacritty")) > b.get(&key("kitty")));
}

#[test]
fn negligible_picks_are_dropped() {
    let mut m = QueryMemory::default();
    m.record("term", &key("kitty"), 0);
    // ~7 half-lives later: p ≈ 0.5 · 2⁻⁷ < MIN_P
    assert_eq!(m.boosts("term", 200).get(&key("kitty")), 0.0);
    // still stored, just below the threshold for ranking
    assert!(p(&m, "term", "kitty") > 0.0);
}

#[test]
fn record_creates_root_and_every_prefix() {
    let mut m = QueryMemory::default();
    m.record("fire", &key("firefox"), 0);
    assert_eq!(m.len(), 5); // "", "f", "fi", "fir", "fire"
}

#[test]
fn unicode_prefixes_split_on_char_boundaries() {
    let mut m = QueryMemory::default();
    m.record("über", &key("uber"), 0); // 'ü' is 2 bytes
    for q in ["ü", "üb", "übe", "über"] {
        approx(p(&m, q, "uber"), 0.5);
    }
}

#[test]
fn truncated_items_still_count_in_total() {
    let mut m = QueryMemory::default();
    for i in 0..10 {
        m.record("x", &key(&i.to_string()), 0);
    }
    assert_eq!(m.nodes["x"].hits.len(), MAX_HITS);
    // all 10 picks stay in the total, even though only MAX_HITS items are kept
    approx(m.nodes["x"].total.at(0), 10.0);
    assert!(p(&m, "x", "9") > 0.0); // the latest pick is kept
}

#[test]
fn new_pick_survives_a_full_node() {
    let mut m = QueryMemory::default();
    for i in 0..MAX_HITS {
        for _ in 0..3 {
            m.record("x", &key(&i.to_string()), 0);
        }
    }
    m.record("x", &key("new"), 0); // weaker than everything already there
    // Space-Saving: replaces the weakest (3.0) and inherits it → 4.0
    approx(p(&m, "x", "new"), 4.0 / (MAX_HITS as f32 * 3.0 + 1.0 + 1.0));
}

#[test]
fn caps_nodes() {
    let mut m = QueryMemory::default();
    for i in 0..MAX_QUERIES + 50 {
        m.record(&format!("q{i}"), &key("a"), 0);
    }
    assert!(m.len() <= MAX_QUERIES);
    assert!(m.nodes.contains_key("q")); // strongest prefix survives
}

#[test]
fn serde_roundtrip() {
    let mut m = QueryMemory::default();
    m.record("firefox", &key("firefox"), 7);
    let json = serde_json::to_string(&m).unwrap();
    let back: QueryMemory = serde_json::from_str(&json).unwrap();
    approx(back.boosts("fire", 7).get(&key("firefox")), 0.5);
}
