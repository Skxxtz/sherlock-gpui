use std::{collections::HashMap, rc::Rc, sync::Arc};

use gpui::{App, AppContext};

use crate::{
    app::LauncherEntity,
    launcher::{Launcher, LauncherConfig, LauncherId},
    ui::model::ItemIdx,
};

/// Every item index in `data`, unfiltered.
/// The order is arbitrary because it follows `HashMap` iteration.
pub(super) fn all_indices(data: &HashMap<LauncherId, Launcher>) -> Arc<[ItemIdx]> {
    data.iter()
        .flat_map(|(id, l)| (0..l.children.len()).map(move |c| (*id, c)))
        .collect()
}

/// Data entity with one empty launcher, for views that fill it at runtime (filesearc, process).
/// `cap` preallocates room for the results.
pub(super) fn single_launcher(
    cfg: &Arc<LauncherConfig>,
    cap: usize,
    cx: &mut App,
) -> LauncherEntity {
    let launcher = Launcher {
        config: cfg.clone(),
        children: Vec::with_capacity(cap),
    };
    cx.new(|_| Rc::new(HashMap::from([(cfg.id(), launcher)])))
}

pub struct CiUtils;
impl CiUtils {
    #[inline]
    pub fn bytes_eq_ci(a: &[u8], b: &[u8]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == *y)
    }

    /// Case-insensitive substring search over raw bytes — no allocation.
    /// Only handles ASCII correctly!!
    #[inline]
    pub fn bytes_contain_ci(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        if needle.len() > haystack.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| {
            w.iter()
                .zip(needle.iter())
                .all(|(h, n)| h.to_ascii_lowercase() == *n)
        })
    }

    #[inline]
    pub fn memrchr_slash(bytes: &[u8]) -> Option<usize> {
        bytes.iter().rposition(|&b| b == b'/')
    }
}
