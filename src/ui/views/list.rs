use std::sync::Arc;

use gpui::{
    AnyElement,
    App,
    Entity,
    IntoElement,
    ParentElement,
    Styled,
    div,
    list,
    px,
    uniform_list,
};

use crate::{
    app::theme::ThemeData,
    ui::{
        launcher::{LauncherView, navigator::views::Grid},
        model::{
            ItemIdx,
            Model,
            data::ModelData,
            item::{SelectionKey, ShellItem},
        },
        views::ShellCtx,
        widgets::RenderableChild,
    },
    utils::config::ConfigGuard,
};

/// Everything an item needs to draw its row.
pub struct RowCtx<'a, Id> {
    /// The item's id in its data.
    pub id: Id,
    /// The item's type-erased identity; select it on click with `Navigator::select_key`.
    pub key: SelectionKey,
    pub selected: bool,
    /// Shortcut number shown on the row, if any.
    pub shortcut: Option<usize>,
    pub query: &'a str,
    pub theme: Arc<ThemeData>,
    /// For click handlers.
    pub shell: Entity<LauncherView>,
}

/// An item that can draw itself as a row (or grid cell) in `list_body` / `grid_body`.
pub trait RowRender<Id>: ShellItem {
    fn render_row(&self, row: &RowCtx<Id>, cx: &mut App) -> AnyElement;
}

impl RowRender<ItemIdx> for RenderableChild {
    fn render_row(&self, row: &RowCtx<ItemIdx>, cx: &mut App) -> AnyElement {
        LauncherView::render_list_item(self, row, cx)
    }
}

/// Vertical list of all visible items, with shortcut numbers.
pub fn list_body<D>(model: &Model<D>, shell: &ShellCtx, cx: &App) -> AnyElement
where
    D: ModelData,
    D::Item: RowRender<D::Id>,
{
    let ids = model.filtered_indices();
    let data = model.data().read(cx).clone();
    let selected_idx = model.selected_index();
    let max_shortcuts = ConfigGuard::read().map_or(5, |c| c.appearance.num_shortcuts) as usize;

    let handle = shell.shell.clone();
    let query = shell.query.clone();
    let theme = shell.theme.clone();

    list(shell.layout.list.clone(), move |idx, _win, cx| {
        let Some(&id) = ids.get(idx) else {
            return div().into_any_element();
        };
        let Some(item) = data.get(id) else {
            return div().into_any_element();
        };

        let shortcut = item
            .shortcut()
            .then(|| {
                let count = ids[..idx.min(ids.len())]
                    .iter()
                    .filter(|&&i| data.get(i).is_some_and(|it| it.shortcut()))
                    .take(max_shortcuts)
                    .count();
                (count < max_shortcuts).then_some(count + 1)
            })
            .flatten();

        let row = RowCtx {
            id,
            key: SelectionKey::new::<D>(&id),
            selected: idx == selected_idx,
            shortcut,
            query: &query,
            theme: theme.clone(),
            shell: handle.clone(),
        };
        item.render_row(&row, cx)
    })
    .pb(px(5.))
    .size_full()
    .into_any_element()
}

/// Grid with `grid.columns` items per row.
pub fn grid_body<D>(model: &Model<D>, grid: &Grid, shell: &ShellCtx) -> AnyElement
where
    D: ModelData,
    D::Item: RowRender<D::Id>,
{
    let ids = model.filtered_indices();
    let data = model.data();
    let selected_idx = model.selected_index();
    let columns = grid.columns;

    let handle = shell.shell.clone();
    let query = shell.query.clone();
    let theme = shell.theme.clone();

    uniform_list("emoji-grid", ids.len().div_ceil(columns), {
        move |range, _win, cx| {
            let data = data.read(cx).clone();
            range
                .map(|row_idx| {
                    div()
                        .flex()
                        .flex_row()
                        .w_full()
                        .gap(px(2.))
                        .children((0..columns).map(|col_idx| {
                            let item_idx = row_idx * columns + col_idx;
                            if let Some(&id) = ids.get(item_idx)
                                && let Some(item) = data.get(id)
                            {
                                let row = RowCtx {
                                    id,
                                    key: SelectionKey::new::<D>(&id),
                                    selected: item_idx == selected_idx,
                                    shortcut: None,
                                    query: &query,
                                    theme: theme.clone(),
                                    shell: handle.clone(),
                                };
                                return div()
                                    .w_0()
                                    .flex_1()
                                    .child(item.render_row(&row, cx))
                                    .into_any_element();
                            }

                            // Empty cell for alignment
                            div().flex_1().into_any_element()
                        }))
                        .into_any_element()
                })
                .collect::<Vec<_>>()
        }
    })
    .gap(px(2.))
    .track_scroll(&grid.scroll)
    .size_full()
    .into_any_element()
}
