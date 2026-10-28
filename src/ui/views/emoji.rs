//! Emoji picker over the static `EMOJIS` table: no per-emoji `RenderableChild`s are built.

use std::sync::Arc;

use gpui::{AnyElement, App, AppContext, Context, Window, px};

use crate::{
    app::data::next_revision,
    features::emoji::{EMOJIS, EmojiEntry},
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::utils::PriorityGuard,
    ui::{
        launcher::{LauncherView, context_menu::ContextMenuAction, navigator::views::Layout},
        model::{
            Model,
            data::ModelData,
            index::{IndexBuilder, Row, SearchIndex},
            item::ShellItem,
            pipeline::{FilterPolicy, GroupRules},
        },
        query_memory::QueryMemoryGuard,
        traits::HandleActions,
        views::{
            Chrome,
            Propagation,
            Query,
            QueryResult,
            ShellCtx,
            SherlockView,
            list::{RowCtx, RowRender, grid_body, list_body},
            query::model_query,
        },
        widgets::emoji::{emoji_actions, emoji_content, render_emoji_cell},
    },
    utils::item_key::ItemKey,
};

/// Position of an emoji in `EMOJIS`. Stable: the table is static.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmojiId(pub u16);

/// `ModelData` over `EMOJIS`, filtered with the emoji launcher's rules.
#[derive(Clone)]
pub struct EmojiSet {
    rules: GroupRules,
    /// Sort weight of the emoji launcher; base priority of every emoji.
    priority: u16,
    revision: u64,
}

impl EmojiSet {
    pub fn new(config: &LauncherConfig) -> Self {
        Self {
            rules: GroupRules::from(config),
            priority: config.priority,
            revision: next_revision(),
        }
    }
}

/// Learned per emoji, so frequently picked emojis rank higher for the same query.
fn emoji_key(entry: &EmojiEntry) -> ItemKey {
    ItemKey::new("emoji", entry.name)
}

impl ModelData for EmojiSet {
    type Id = EmojiId;
    type Item = EmojiEntry;

    #[inline(always)]
    fn revision(&self) -> u64 {
        self.revision
    }

    #[inline]
    fn get(&self, id: EmojiId) -> Option<&EmojiEntry> {
        EMOJIS.get(id.0 as usize)
    }

    fn all_ids(&self) -> Arc<[EmojiId]> {
        (0..EMOJIS.len() as u16).map(EmojiId).collect()
    }

    fn build_index(&self) -> SearchIndex<EmojiId> {
        let mut b = IndexBuilder::new(self.revision);
        let group = b.group(self.rules.clone());
        for (i, entry) in EMOJIS.iter().enumerate() {
            let key = emoji_key(entry);
            let prio = PriorityGuard::new(self.priority, QueryMemoryGuard::popularity_count(&key));
            b.push(
                group,
                Row::new(EmojiId(i as u16), entry.keywords, prio).key(Some(key)),
            );
        }
        b.finish()
    }

    /// Emojis are never contextual.
    fn based_show<C: AppContext>(&self, _id: EmojiId, _query: &str, _cx: &mut C) -> Option<bool> {
        None
    }
}

impl ShellItem for EmojiEntry {
    fn exec(&self, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::Copy {
            content: emoji_content(self),
        })
    }

    fn actions(&self, _cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let index = EMOJIS.iter().position(|e| std::ptr::eq(e, self))?;
        emoji_actions(index)
    }

    fn has_actions(&self, _cx: &mut App) -> bool {
        self.skin_tones > 0
    }

    fn action_exec(&self, action: Arc<ContextMenuAction>, _cx: &mut App) -> ExecMode {
        match action.as_ref() {
            ContextMenuAction::Emoji(emj) => emj
                .entry()
                .map(|entry| ExecMode::Copy {
                    content: emoji_content(entry),
                })
                .unwrap_or_default(),
            _ => ExecMode::default(),
        }
    }

    fn pick_key(&self) -> Option<ItemKey> {
        Some(emoji_key(self))
    }
}

impl RowRender<EmojiId> for EmojiEntry {
    fn render_row(&self, row: &RowCtx<EmojiId>, _cx: &mut App) -> AnyElement {
        let content = render_emoji_cell(self, row.selected, &row.theme);
        LauncherView::render_row_frame(row, false, content)
    }
}

/// Number of emojis per row in the grid.
const COLUMNS: usize = 6;

pub struct EmojiView {
    pub model: Model<EmojiSet>,
}

impl EmojiView {
    pub fn new(config: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let data = EmojiSet::new(&config);
        let all = data.all_ids();
        Self {
            model: Model::new(cx.new(|_| data), all, FilterPolicy::All),
        }
    }
}

impl HandleActions for EmojiView {
    const UNHANDLED: Propagation = Propagation::Continue;
}

impl SherlockView for EmojiView {
    type Data = EmojiSet;

    fn model(&self) -> Option<&Model<EmojiSet>> {
        Some(&self.model)
    }

    fn model_mut(&mut self) -> Option<&mut Model<EmojiSet>> {
        Some(&mut self.model)
    }

    fn layout(&self) -> Layout {
        Layout::grid(self.model.len(), px(100.), COLUMNS)
    }

    fn chrome(&self) -> Chrome {
        Chrome::NO_SIDEBAR
    }

    fn render_body(
        &mut self,
        shell: &ShellCtx,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &shell.layout.grid {
            Some(grid) => grid_body(&self.model, grid, shell),
            None => list_body(&self.model, shell, cx),
        }
    }

    fn on_query(&mut self, query: &Query, cx: &mut Context<Self>) -> QueryResult {
        model_query(&mut self.model, query, cx, |v: &mut Self| &mut v.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ui::{model::pipeline::FilterCtx, query_memory::Boosts},
        utils::config::HomeType,
    };

    fn set() -> EmojiSet {
        EmojiSet::new(&LauncherConfig {
            priority: 1,
            home: HomeType::Home,
            ..Default::default()
        })
    }

    fn search(data: &EmojiSet, query: &str) -> Vec<EmojiId> {
        let index = data.build_index();
        let gates = index.gates(&FilterCtx::new(FilterPolicy::All, "all", query));
        let scored = index.score(&index.pure, &gates, query, &Boosts::default(), |_| None);
        crate::ui::model::pipeline::sort_and_limit(scored, FilterPolicy::All).to_vec()
    }

    #[test]
    fn one_row_per_emoji() {
        let data = set();
        let index = data.build_index();
        assert_eq!(index.pure.len(), EMOJIS.len());
        assert!(index.contextual.is_empty());
        assert_eq!(data.all_ids().len(), EMOJIS.len());
    }

    #[test]
    fn ids_resolve_to_entries() {
        let data = set();
        assert!(std::ptr::eq(data.get(EmojiId(0)).unwrap(), &EMOJIS[0]));
        assert!(data.get(EmojiId(u16::MAX)).is_none());
    }

    #[test]
    fn finds_by_keyword() {
        let data = set();
        let hits = search(&data, "thumbs up");
        let first = hits.first().and_then(|&id| data.get(id)).map(|e| e.name);
        assert_eq!(first, Some("Thumbs Up"));
    }

    #[test]
    fn empty_query_shows_all_on_home() {
        assert_eq!(search(&set(), "").len(), EMOJIS.len());
    }
}
