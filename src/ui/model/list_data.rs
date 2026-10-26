#![allow(dead_code)]

use std::{rc::Rc, sync::Arc};

use gpui::AppContext;

use crate::{
    app::data::next_revision,
    loader::utils::PriorityGuard,
    ui::model::{
        data::ModelData,
        index::{IndexBuilder, Row, SearchIndex},
        pipeline::GroupRules,
    },
    utils::item_key::ItemKey,
};

/// What `ListData` needs to know about an item to filter and sort it.
pub trait ListItem: 'static {
    /// Text matched against the query. Should already be lowercase.
    fn search(&self) -> &str;

    /// Sort weight; lower first. Ties are broken by match quality.
    fn priority(&self) -> u16 {
        1
    }

    /// Key for query-memory boosts (items the user picks often rank higher).
    fn key(&self) -> Option<ItemKey> {
        None
    }
}

/// Position of an item in a `ListData`.
///
/// Stable as long as the list isn't replaced; after `set`, ids refer to the new items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ListId(pub u32);

/// A plain list of items, filtered as one `GroupRules::plain` group.
#[derive(Debug)]
pub struct ListData<T> {
    items: Rc<[T]>,
    revision: u64,
}

// manual impl: `T` itself doesn't need to be `Clone`
impl<T> Clone for ListData<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            revision: self.revision,
        }
    }
}

impl<T> ListData<T> {
    pub fn new(items: impl Into<Rc<[T]>>) -> Self {
        Self {
            items: items.into(),
            revision: next_revision(),
        }
    }

    /// Replaces all items. Bumps the revision, so the search index rebuilds.
    pub fn set(&mut self, items: impl Into<Rc<[T]>>) {
        self.items = items.into();
        self.revision = next_revision();
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T> Default for ListData<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<T: ListItem> ModelData for ListData<T> {
    type Id = ListId;
    type Item = T;

    #[inline(always)]
    fn revision(&self) -> u64 {
        self.revision
    }

    #[inline]
    fn get(&self, id: ListId) -> Option<&T> {
        self.items.get(id.0 as usize)
    }

    fn all_ids(&self) -> Arc<[ListId]> {
        (0..self.items.len() as u32).map(ListId).collect()
    }

    fn build_index(&self) -> SearchIndex<ListId> {
        let mut b = IndexBuilder::new(self.revision);
        let group = b.group(GroupRules::plain());
        for (i, item) in self.items.iter().enumerate() {
            b.push(
                group,
                Row::new(
                    ListId(i as u32),
                    item.search(),
                    PriorityGuard::new(item.priority(), 0),
                )
                .key(item.key()),
            );
        }
        b.finish()
    }

    /// List items are never contextual, so this is never called.
    fn based_show<C: AppContext>(&self, _id: ListId, _query: &str, _cx: &mut C) -> Option<bool> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        model::pipeline::{self, FilterCtx, FilterPolicy},
        query_memory::Boosts,
    };

    struct Entry(&'static str, u16);
    impl ListItem for Entry {
        fn search(&self) -> &str {
            self.0
        }
        fn priority(&self) -> u16 {
            self.1
        }
    }

    fn data() -> ListData<Entry> {
        ListData::new(vec![
            Entry("firefox", 1),
            Entry("files", 1),
            Entry("chrome", 1),
            Entry("fish", 0),
        ])
    }

    /// The same steps `FilterJob` runs, minus the parts that need gpui.
    fn filter(data: &ListData<Entry>, query: &str, mode: &str) -> Vec<ListId> {
        let index = data.build_index();
        let gates = index.gates(&FilterCtx::new(FilterPolicy::All, mode, query));
        let scored = index.score(&index.pure, &gates, query, &Boosts::default(), |_| None);
        pipeline::sort_and_limit(scored, FilterPolicy::All).to_vec()
    }

    #[test]
    fn ids_and_lookup() {
        let d = data();
        assert_eq!(&*d.all_ids(), &[ListId(0), ListId(1), ListId(2), ListId(3)]);
        assert_eq!(d.get(ListId(2)).map(|e| e.0), Some("chrome"));
        assert!(d.get(ListId(9)).is_none());
    }

    #[test]
    fn index_is_one_pure_group() {
        let index = data().build_index();
        assert_eq!(index.pure.len(), 4);
        assert!(index.contextual.is_empty());
        assert!(index.pure.iter().all(|r| r.group == 0));
    }

    #[test]
    fn filters_by_query() {
        let hits = filter(&data(), "fi", "all");
        assert!(hits.contains(&ListId(0)) && hits.contains(&ListId(1)));
        assert!(!hits.contains(&ListId(2)));
    }

    #[test]
    fn lower_priority_sorts_first() {
        assert_eq!(filter(&data(), "fi", "all").first(), Some(&ListId(3)));
    }

    #[test]
    fn plain_group_shows_on_home_hidden_in_alias_mode() {
        assert_eq!(filter(&data(), "", "all").len(), 4);
        assert_eq!(filter(&data(), "", "some-alias").len(), 0);
    }

    #[test]
    fn set_replaces_items_and_bumps_revision() {
        let mut d = data();
        let index = d.build_index();
        d.set(vec![Entry("only", 1)]);
        assert!(index.is_stale(&d));
        assert_eq!(d.len(), 1);
    }
}
