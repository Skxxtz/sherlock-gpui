use std::{collections::HashMap, sync::Arc};

use gpui::{App, AppContext};

use crate::{
    app::{LauncherEntity, data::LauncherData},
    launcher::{Children, Launcher, LauncherConfig, LauncherId},
    ui::model::ItemIdx,
};

/// Every item index in `data`, unfiltered.
/// The order is arbitrary because it follows `HashMap` iteration.
pub(super) fn all_indices(data: &HashMap<LauncherId, Launcher>) -> Arc<[ItemIdx]> {
    data.iter()
        .flat_map(|(&id, l)| l.children.ids().map(move |c| (id, c)))
        .collect()
}

/// Data entity with one empty launcher, for views that fill it at runtime (file search, process).
/// `cap` preallocates room for the results.
pub(super) fn single_launcher(
    cfg: &Arc<LauncherConfig>,
    cap: usize,
    cx: &mut App,
) -> LauncherEntity {
    let launcher = Launcher {
        config: cfg.clone(),
        children: Children::with_capacity(cap),
    };
    cx.new(|_| LauncherData::new(HashMap::from([(cfg.id(), launcher)])))
}
