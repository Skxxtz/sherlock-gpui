//! Estimates P(item | query) from the user's own picks.
//!
//! # Model
//!
//! When a user picks item `i` after typing query `q`, it's evidence that the user wanted to pick
//! `i` at **every prefix** of `q` (Whoever typed "firefox" and picked Firefox already meant
//! Firefox at "f", "fi", and so on.).
//!
//! `record`then remembers the pick for each prefix and `boosts(q)` is a single lookup of node `q`.
//!
//! Per node a decayed count `c(i, q)` and the total `c(q)` are remembered. The decay is based on a
//! half-life, so old habits gradually fade.
//!
//! # Estimation
//!
//! The estimated probabilty of picking `i` given the query `q` (P(item | query)) is estimated using
//! the following equation:
//!
//! ```math
//! P̂(i | q) = c(i, q) / (c(q) + α)
//! ```
//! Where `α` is a pseudo-count for anything that wasn't picked yet.
//!
//! Little evidence: the estimate stays low (1 pick → 0.5)
//! Much evidence: approaches the true share
//!
//! Counts decay with a half-life, so old habits fade.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::{ALPHA, HALF_LIFE_DAYS, MAX_HITS, MAX_QUERIES, MIN_P};
use crate::utils::item_key::ItemKey;

/// An exponentially decaying count.
/// The decay follows this equation:
/// ```math
/// value(today) = Σᵢ 0.5^((today − tᵢ) / H)
/// ```
/// Where `H` equals `HALF_LIFE_DAYS
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) struct Decayed {
    weight: f32,
    day: u32,
}

impl Decayed {
    pub(super) fn at(self, today: u32) -> f32 {
        let age = today.saturating_sub(self.day) as f32;
        self.weight * 0.5f32.powf(age / HALF_LIFE_DAYS)
    }

    /// Day of the last update.
    pub(super) fn day(self) -> u32 {
        self.day
    }

    /// Sums the decayed history of calls with a new (undecayed) call (= decayed + amount).
    fn add(&mut self, amount: f32, today: u32) {
        self.weight = self.at(today) + amount;
        self.day = today;
    }
}

/// `MAX_HITS` plus one slot for the moment between push and eviction.
type Hits = SmallVec<[Hit; MAX_HITS + 1]>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Hit {
    pub(super) key: ItemKey,
    pub(super) count: Decayed,
}

/// Evidence for one prefix.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Node {
    /// `c(q)`: all picks under this prefix, including items that were
    /// dropped from `hits`, so probabilities stay honest after truncation.
    pub(super) total: Decayed,
    /// `c(i, q)` for the strongest few items.
    pub(super) hits: Hits,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct QueryMemory {
    /// Normalized prefix → evidence.
    pub(super) nodes: BTreeMap<String, Node>,
}

/// Per-search lookup table: item → `P̂(item | query)`.
#[derive(Default, Debug)]
pub struct Boosts(SmallVec<[(ItemKey, f32); 8]>);

impl Boosts {
    /// `P̂(item | query)`, or 0 for items never picked under this query.
    #[inline]
    pub fn get(&self, key: &ItemKey) -> f32 {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map_or(0.0, |&(_, p)| p)
    }
}

/// Days since the Unix epoch.
pub fn today() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| (d.as_secs() / 86_400) as u32)
}

pub(super) fn normalize(query: &str) -> Option<String> {
    let q = query.trim().to_lowercase();
    (!q.is_empty()).then_some(q)
}

impl QueryMemory {
    /// Remembers that `key` was picked after typing `query`.
    /// Returns `false` if nothing was learned (empty query).
    pub fn record(&mut self, query: &str, key: &ItemKey, today: u32) -> bool {
        let Some(q) = normalize(query) else {
            return false;
        };

        // every prefix on a char boundary: "fi", "fir", … "firefox"
        let ends = q.char_indices().map(|(i, c)| i + c.len_utf8());
        for end in ends {
            let node = self.nodes.entry(q[..end].to_owned()).or_default();
            node.total.add(1.0, today);

            match node.hits.iter().position(|h| &h.key == key) {
                // already tracked: count up
                Some(i) => node.hits[i].count.add(1.0, today),

                // new item, room left: start at 1
                None if node.hits.len() < MAX_HITS => node.hits.push(Hit {
                    key: key.clone(),
                    count: Decayed {
                        weight: 1.0,
                        day: today,
                    },
                }),

                // new item, node full: replace the weakest and inherit its weight
                None => {
                    let (weakest, inherited) = node
                        .hits
                        .iter()
                        .map(|h| h.count.at(today))
                        .enumerate()
                        .min_by(|(_, a), (_, b)| a.total_cmp(b))
                        .expect("node is full, so not empty");
                    node.hits[weakest] = Hit {
                        key: key.clone(),
                        count: Decayed {
                            weight: inherited + 1.0,
                            day: today,
                        },
                    };
                }
            }
        }

        self.evict(today);
        true
    }

    /// `P̂(item | query)` for the items seen under this prefix.
    /// `P̂(i | q) = c(i, q) / (c(q) + α)`
    pub fn probabilities(&self, query: &str, today: u32) -> SmallVec<[(ItemKey, f32); 8]> {
        let Some(node) = normalize(query).and_then(|q| self.nodes.get(&q)) else {
            return SmallVec::new();
        };
        let denom = node.total.at(today) + ALPHA;
        node.hits
            .iter()
            .map(|h| (h.key.clone(), h.count.at(today) / denom))
            .collect()
    }

    /// `P̂(item | query)` for this search.
    pub fn boosts(&self, query: &str, today: u32) -> Boosts {
        let mut ps = self.probabilities(query, today);
        ps.retain(|(_, p)| *p >= MIN_P);
        Boosts(ps)
    }

    /// Drops the weakest prefixes once over the cap. Short prefixes collect
    /// the most evidence, so they survive longest.
    fn evict(&mut self, today: u32) {
        let excess = self.nodes.len().saturating_sub(MAX_QUERIES);
        if excess == 0 {
            return;
        }
        let mut by_strength: Vec<(f32, String)> = self
            .nodes
            .iter()
            .map(|(q, n)| (n.total.at(today), q.clone()))
            .collect();
        by_strength.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, q) in by_strength.into_iter().take(excess) {
            self.nodes.remove(&q);
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
