use std::sync::Arc;

use gpui::{AppContext, SharedString};

use crate::{
    QUERY_MEMORY,
    app::data::LauncherData,
    loader::utils::PriorityGuard,
    ui::{
        model::{
            ItemIdx,
            data::ModelData,
            pipeline::{self, FilterCtx, FilterPolicy, GroupGate, GroupRules, Scored},
        },
        query_memory::Boosts,
        traits::RenderableChildDelegate,
        utils::search::SherlockSearch,
    },
    utils::item_key::ItemKey,
};

/// Everything filtering needs to know about one item.
#[derive(Clone)]
pub struct SearchRow<Id> {
    pub id: Id,
    /// Index into the index's groups.
    pub group: u32,
    pub text: Arc<str>,
    pub key: Option<ItemKey>,
    pub prio: PriorityGuard,
}

/// Thread-safe snapshot of one data revision.
pub struct SearchIndex<Id> {
    revision: u64,
    groups: Box<[GroupRules]>,
    /// Filtered off the main thread.
    pub pure: Box<[SearchRow<Id>]>,
    /// Need `cx` (`based_show`); filtered on the main thread.
    pub contextual: Box<[SearchRow<Id>]>,
}

impl<Id: Copy> SearchIndex<Id> {
    /// For `ModelData` implementations that build their own rows.
    pub fn new(
        revision: u64,
        groups: Vec<GroupRules>,
        pure: Vec<SearchRow<Id>>,
        contextual: Vec<SearchRow<Id>>,
    ) -> Self {
        Self {
            revision,
            groups: groups.into(),
            pure: pure.into(),
            contextual: contextual.into(),
        }
    }

    /// The data revision this index was built from.
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True if the data changed since this index was built.
    #[inline(always)]
    pub fn is_stale<D: ModelData>(&self, data: &D) -> bool {
        self.revision != data.revision()
    }

    /// Rules 1-4 for every group. Index with `SearchRow::group`.
    pub fn gates(&self, fcx: &FilterCtx) -> Arc<[GroupGate]> {
        self.groups.iter().map(|g| fcx.gate(g)).collect()
    }

    /// Rules 4-6 + scoring for `rows`.
    ///
    /// `based_show` is only called for rows in a `Match` group; `|_| None` for pure rows.
    pub fn score(
        &self,
        rows: &[SearchRow<Id>],
        gates: &[GroupGate],
        query: &str,
        boosts: &Boosts,
        mut based_show: impl FnMut(&SearchRow<Id>) -> Option<bool>,
    ) -> Vec<Scored<Id>> {
        rows.iter()
            .filter(|row| {
                pipeline::admit(
                    gates[row.group as usize],
                    || based_show(row),
                    || row.text.fuzzy_match(query),
                )
            })
            .map(|row| {
                let boost = row.key.as_ref().map_or(0.0, |k| boosts.get(k));
                Scored {
                    index: row.id,
                    prio: row.prio.get().sort_key(query, &row.text, boost),
                    limit: self.groups[row.group as usize].limit,
                }
            })
            .collect()
    }
}

/// Builds a `SearchIndex` without assembling groups and rows by hand.
///
/// ```ignore
/// let mut b = IndexBuilder::new(data.revision());
/// let g = b.group(GroupRules::plain());
/// for (id, item) in items {
///     b.push(g, Row::new(id, item.name(), PriorityGuard::new(1, 0)));
/// }
/// let index = b.finish();
/// ```
pub struct IndexBuilder<Id> {
    revision: u64,
    groups: Vec<GroupRules>,
    pure: Vec<SearchRow<Id>>,
    contextual: Vec<SearchRow<Id>>,
}

/// Handle to a group added with `IndexBuilder::group`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupId(u32);

/// One item for `IndexBuilder::push`.
pub struct Row<Id> {
    id: Id,
    text: Arc<str>,
    key: Option<ItemKey>,
    prio: PriorityGuard,
    contextual: bool,
}

impl<Id> Row<Id> {
    /// `text` is matched against the query; it should already be lowercase.
    pub fn new(id: Id, text: impl Into<Arc<str>>, prio: PriorityGuard) -> Self {
        Self {
            id,
            text: text.into(),
            key: None,
            prio,
            contextual: false,
        }
    }

    /// Key for query-memory boosts.
    pub fn key(mut self, key: Option<ItemKey>) -> Self {
        self.key = key;
        self
    }

    /// The item needs `ModelData::based_show` (and so `cx`); filtered on the main thread.
    pub fn contextual(mut self, contextual: bool) -> Self {
        self.contextual = contextual;
        self
    }
}

impl<Id: Copy> IndexBuilder<Id> {
    pub fn new(revision: u64) -> Self {
        Self {
            revision,
            groups: Vec::new(),
            pure: Vec::new(),
            contextual: Vec::new(),
        }
    }

    /// Adds a group; items pushed into it share its rules (mode, home, limit).
    pub fn group(&mut self, rules: GroupRules) -> GroupId {
        self.groups.push(rules);
        GroupId(self.groups.len() as u32 - 1)
    }

    pub fn push(&mut self, group: GroupId, row: Row<Id>) -> &mut Self {
        let Row {
            id,
            text,
            key,
            prio,
            contextual,
        } = row;
        let row = SearchRow {
            id,
            group: group.0,
            text,
            key,
            prio,
        };
        if contextual {
            self.contextual.push(row);
        } else {
            self.pure.push(row);
        }
        self
    }

    pub fn finish(self) -> SearchIndex<Id> {
        SearchIndex::new(self.revision, self.groups, self.pure, self.contextual)
    }
}

impl SearchIndex<ItemIdx> {
    pub fn build(data: &LauncherData) -> Self {
        let mut b = IndexBuilder::new(data.revision());

        for (&launcher_id, launcher) in data.iter() {
            let group = b.group(GroupRules::from(launcher.config.as_ref()));
            for (local_id, child) in launcher.children.iter_with_ids() {
                b.push(
                    group,
                    Row::new((launcher_id, local_id), child.search(), child.prio.clone())
                        .key(child.item_key())
                        .contextual(child.has_based_show()),
                );
            }
        }

        b.finish()
    }
}

/// One query's worth of filtering, ready to finish on any thread.
///
/// Built on the main thread (contextual items already scored); `run` scores the pure items and
/// sorts/limits everything.
pub struct FilterJob<Id> {
    index: Arc<SearchIndex<Id>>,
    gates: Arc<[GroupGate]>,
    boosts: Boosts,
    policy: FilterPolicy,
    query: SharedString,
    contextual: Vec<Scored<Id>>,
}

impl<Id: Copy> FilterJob<Id> {
    /// Main-thread half: rules 1-4, boosts and the contextual items (needs `cx`).
    pub fn prepare<D, C>(
        index: Arc<SearchIndex<Id>>,
        data: &D,
        policy: FilterPolicy,
        mode: &str,
        query: SharedString,
        cx: &mut C,
    ) -> Self
    where
        D: ModelData<Id = Id>,
        C: AppContext,
    {
        let gates = index.gates(&FilterCtx::new(policy, mode, &query));
        let boosts = QUERY_MEMORY
            .get()
            .and_then(|l| l.read().ok())
            .map(|m| m.boosts(&query))
            .unwrap_or_default();

        // [Rule 5]
        let contextual = index.score(&index.contextual, &gates, &query, &boosts, |row| {
            data.based_show(row.id, &query, cx)
        });

        Self {
            index,
            gates,
            boosts,
            policy,
            query,
            contextual,
        }
    }

    /// Thread-safe half: scores the pure items, then sorts and limits everything.
    pub fn run(self) -> Arc<[Id]> {
        let Self {
            index,
            gates,
            boosts,
            policy,
            query,
            mut contextual,
        } = self;
        let mut results = index.score(&index.pure, &gates, &query, &boosts, |_| None);
        results.append(&mut contextual);
        pipeline::sort_and_limit(results, policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_send<T: Send>() {}

    #[test]
    fn index_and_job_are_thread_safe() {
        assert_send_sync::<SearchIndex<ItemIdx>>();
        assert_send_sync::<SearchRow<ItemIdx>>();
        assert_send::<FilterJob<ItemIdx>>();
    }

    #[test]
    fn empty_data_builds_empty_index() {
        let data = LauncherData::default();
        let idx = SearchIndex::build(&data);
        assert!(idx.pure.is_empty() && idx.contextual.is_empty());
        assert!(!idx.is_stale(&data));
    }

    #[test]
    fn index_goes_stale_on_mutation() {
        let mut data = LauncherData::default();
        let idx = SearchIndex::build(&data);
        data.make_mut();
        assert!(idx.is_stale(&data));
    }
}
