use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, list, px, uniform_list};

use crate::{
    launcher::LauncherValues,
    ui::{
        launcher::{LauncherView, navigator::views::Grid},
        model::Model,
        utils::selection::Selection,
        views::ShellCtx,
    },
    utils::config::ConfigGuard,
};

pub fn list_body(model: &Model, shell: &ShellCtx, cx: &App) -> AnyElement {
    let indices = model.filtered_indices();
    let data = model.data().read(cx).clone();
    let selected_idx = model.selected_index();
    let max_shortcuts = ConfigGuard::read().map_or(5, |c| c.appearance.num_shortcuts) as usize;

    let handle = shell.shell.clone();
    let query = shell.query.clone();
    let theme = shell.theme.clone();

    list(shell.layout.list.clone(), move |idx, _win, cx| {
        let Some(&data_idx) = indices.get(idx) else {
            return div().into_any_element();
        };
        let Some(child) = data.child(data_idx) else {
            return div().into_any_element();
        };
        let shortcut_idx = child
            .shortcut()
            .then(|| {
                let count = indices[..idx.min(indices.len())]
                    .iter()
                    .filter(|&&i| data.get(&i.0).is_some_and(|l| l.config.shortcut))
                    .take(max_shortcuts)
                    .count();
                (count < max_shortcuts).then_some(count + 1)
            })
            .flatten();

        LauncherView::render_list_item(
            child,
            shortcut_idx,
            Selection::new(data_idx, idx == selected_idx),
            &query,
            theme.clone(),
            handle.clone(),
            cx,
        )
    })
    .pb(px(5.))
    .size_full()
    .into_any_element()
}

pub fn grid_body(model: &Model, grid: &Grid, shell: &ShellCtx) -> AnyElement {
    let indices = model.filtered_indices();
    let data = model.data();
    let selected_idx = model.selected_index();
    let columns = grid.columns;

    let handle = shell.shell.clone();
    let query = shell.query.clone();
    let theme = shell.theme.clone();

    uniform_list("emoji-grid", indices.len().div_ceil(columns), {
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
                            if let Some(&data_idx) = indices.get(item_idx)
                                && let Some(child) = data.child(data_idx)
                            {
                                return div()
                                    .w_0()
                                    .flex_1()
                                    .child(LauncherView::render_list_item(
                                        child,
                                        None,
                                        Selection::new(data_idx, item_idx == selected_idx),
                                        &query,
                                        theme.clone(),
                                        handle.clone(),
                                        cx,
                                    ))
                                    .into_any_element();
                            }

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
