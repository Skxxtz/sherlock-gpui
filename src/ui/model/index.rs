use std::sync::Arc;

use crate::{
    app::data::LauncherData,
    loader::utils::PriorityGuard,
    ui::{
        model::{
            ItemIdx,
            pipeline::{self, FilterCtx, GroupGate, GroupRules, Scored},
        },
        query_memory::Boosts,
        traits::RenderableChildDelegate,
        utils::search::SherlockSearch,
    },
    utils::item_key::ItemKey,
};

#[derive(Clone)]
pub struct SearchRow {
    pub id: ItemIdx,
    pub group: u32,
    pub text: Arc<str>,
    pub key: Option<ItemKey>,
    pub prio: PriorityGuard,
}

/// Snapshot of one `LauncherData` revision.
pub struct SearchIndex {
    revision: u64,
    groups: Box<[GroupRules]>,
    pub pure: Box<[SearchRow]>,
    pub contextual: Box<[SearchRow]>,
}

impl SearchIndex {
    pub fn build(data: &LauncherData) -> Self {
        let mut groups = Vec::with_capacity(data.len());
        let mut pure = Vec::new();
        let mut contextual = Vec::new();

        for (&launcher_id, launcher) in data.iter() {
            let group = groups.len() as u32;
            groups.push(GroupRules::from(launcher.config.as_ref()));

            for (local_id, child) in launcher.children.iter_with_ids() {
                let row = SearchRow {
                    id: (launcher_id, local_id),
                    group,
                    text: Arc::from(child.search()),
                    key: child.item_key(),
                    prio: child.prio.clone(),
                };
                if child.has_based_show() {
                    contextual.push(row);
                } else {
                    pure.push(row);
                }
            }
        }

        Self {
            revision: data.revision(),
            groups: groups.into(),
            pure: pure.into(),
            contextual: contextual.into(),
        }
    }

    /// The `LauncherData` revision this index was built from.
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True if `data` changed since this index was built.
    #[inline(always)]
    pub fn is_stale(&self, data: &LauncherData) -> bool {
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
        rows: &[SearchRow],
        gates: &[GroupGate],
        query: &str,
        boosts: &Boosts,
        mut based_show: impl FnMut(&SearchRow) -> Option<bool>,
    ) -> Vec<Scored<ItemIdx>> {
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

/// One query's woth of filtering, ready to finish on any thread.
///
/// Build on the main thread (contexttual items already scored); `run` scores the pure items and
/// sorts/limits everything.
pub struct FilterJob {
    pub index: Arc<SearchIndex>,
    pub gates: Arc<[GroupGate]>,
    pub boosts: Boosts,
    pub policy: pipeline::FilterPolicy,
    pub query: gpui::SharedString,
    pub contextual: Vec<Scored<ItemIdx>>,
}

impl FilterJob {
    pub fn run(self) -> Arc<[ItemIdx]> {
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

    #[test]
    fn index_and_job_are_thread_safe() {
        assert_send_sync::<SearchIndex>();
        assert_send_sync::<SearchRow>();
        fn assert_send<T: Send>() {}
        assert_send::<FilterJob>();
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
