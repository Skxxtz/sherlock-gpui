use crate::app::LauncherWeakEntity;
use crate::launcher::file_launcher::FileLauncher;
use crate::launcher::{LauncherConfig, variant_type::LauncherType};
use crate::ui::launcher::LauncherView;
use crate::ui::model::utils::CiUtils;
use crate::ui::widgets::RenderableChild;
use crate::ui::widgets::file::FileData;
use crate::utils::files::expand_path;
use gpui::{App, SharedString, Task, WeakEntity};
use std::env::home_dir;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc;
use utils::{FileResult, ResultHeap};

mod backends;
mod utils;
pub mod view;

pub use backends::FileSearchBackend;

#[derive(Default)]
pub struct FileSearchModel {
    backend: FileSearchBackend,
    launcher: Arc<LauncherConfig>,
    results: Vec<FileResult>,
    poll_interval: u64,
    paths: Arc<Vec<PathBuf>>,
    cancel_tx: Option<mpsc::Sender<()>>,
    _poll_task: Option<Task<()>>,
}

pub(super) const MAX_SEARCH_DEPTH: usize = 6;

impl FileSearchModel {
    pub fn new(launcher: Arc<LauncherConfig>, dir: Option<SharedString>) -> Self {
        if let LauncherType::Files(FileLauncher {
            ref backend,
            ref loc,
            max_results,
            poll_interval,
        }) = launcher.launcher_type
        {
            let home = home_dir().unwrap_or(PathBuf::from("/"));
            let paths = Arc::new(vec![
                dir.map(|d| expand_path(d.as_str(), &home))
                    .unwrap_or(expand_path(loc.as_str(), &home)),
            ]);

            Self {
                backend: backend.clone(),
                poll_interval,
                paths,
                launcher,
                results: Vec::with_capacity(max_results),
                cancel_tx: None,
                _poll_task: None,
            }
        } else {
            Self {
                launcher,
                results: Vec::with_capacity(0),
                ..Default::default()
            }
        }
    }

    pub fn search(
        &mut self,
        query_lower: Arc<str>,
        result_entity: LauncherWeakEntity,
        launcher_weak: WeakEntity<LauncherView>,
        cx: &mut App,
    ) {
        // Dropping to cacel running tasks
        self.cancel_tx = None;
        self._poll_task = None;
        self.results.clear();

        let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
        self.cancel_tx = Some(cancel_tx);

        let (result_tx, mut result_rx) = mpsc::channel::<Vec<FileResult>>(32);

        let backend = self.backend.clone();
        let cap = self.results.capacity();
        let paths = Arc::clone(&self.paths);
        std::thread::spawn({
            let query_lower = Arc::clone(&query_lower);
            move || {
                let mut heap = ResultHeap::new(cap);
                let completed =
                    backend.search(query_lower, paths, &mut heap, cancel_rx, &result_tx);
                if completed {
                    let _ = result_tx.try_send(heap.snapshot());
                }
                // result_tx drops here → result_rx.recv() returns None → poll task exits
            }
        });

        let launcher = Arc::clone(&self.launcher);
        let poll_interval = self.poll_interval;
        let poll_task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(poll_interval))
                    .await;

                let mut latest: Option<Vec<FileResult>> = None;
                let mut channel_open = true;

                loop {
                    match result_rx.try_recv() {
                        Ok(snap) => {
                            latest = Some(snap);
                        }
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        // Sender dropped — thread has exited (completed or cancelled)
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            channel_open = false;
                            break;
                        }
                    }
                }

                if let Some(snapshot) = latest {
                    let count = snapshot.len();
                    let children = snapshot
                        .into_iter()
                        .map(|r| RenderableChild::File {
                            launcher: Arc::clone(&launcher),
                            inner: FileData::new(r.path.clone()).with_icon_name(r.get_icon_name()),
                        })
                        .collect::<Vec<_>>();

                    let indices: Arc<[(usize, usize)]> =
                        (0..count).map(|c| (0, c)).collect::<Vec<_>>().into();

                    if let Some(view) = launcher_weak.upgrade() {
                        cx.update({
                            let query_lower = Arc::clone(&query_lower);
                            |cx| {
                                view.update(cx, |this, cx| {
                                    // Swap the entity data directly
                                    if let Some(entity) = result_entity.upgrade() {
                                        entity.update(cx, |e, _| {
                                            let data = Rc::make_mut(e);
                                            if let Some(launcher) = data.get_mut(0) {
                                                launcher.children = children;
                                            }
                                        });
                                    }
                                    // Reuse the exact same apply_results path as regular search
                                    this.apply_results(indices, query_lower, false, cx);
                                });
                            }
                        });
                    }
                }

                if !channel_open {
                    break;
                }
            }
        });

        self._poll_task = Some(poll_task);
    }
}

pub(super) struct FileSearchUtility;
impl FileSearchUtility {
    /// Returns true if `entry` is a hidden file or inside a hidden directory.
    #[inline]
    fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        entry.file_name().as_encoded_bytes().first().copied() == Some(b'.')
    }

    // Comparing with already lower-cased query,
    // using a zero-alloc case-insensitive comparator
    #[inline]
    fn score_file_ci(name_bytes: &[u8], query: &str) -> f32 {
        let q = query.as_bytes();
        let len = name_bytes.len();
        let qlen = q.len();

        let eq = len == qlen && CiUtils::bytes_eq_ci(name_bytes, q);
        let ends = len > qlen && CiUtils::bytes_eq_ci(&name_bytes[len - qlen..], q);
        let starts = len > qlen && CiUtils::bytes_eq_ci(&name_bytes[..qlen], q);

        if eq {
            return 0.0;
        }
        if ends {
            return 0.05;
        }
        if starts {
            return 0.1 + 0.1 * (1.0 - qlen as f32 / len as f32);
        }
        if CiUtils::bytes_contain_ci(name_bytes, q) {
            return 0.4;
        }
        0.8
    }

    #[inline]
    fn score_path(path_bytes: &[u8], query: &[u8]) -> f32 {
        let plen = path_bytes.len();
        let qlen = query.len();

        if CiUtils::bytes_eq_ci(path_bytes, query) {
            return 0.0;
        }

        if plen > qlen {
            let tail = &path_bytes[plen - qlen..];
            if CiUtils::bytes_eq_ci(tail, query) {
                let prev = path_bytes[plen - qlen - 1];
                if prev == b'/' || prev == b'\\' {
                    return 0.05;
                }
            }
        }
        if CiUtils::bytes_contain_ci(path_bytes, query) {
            return 0.3 + 0.1 * (1.0 - qlen as f32 / plen as f32);
        }
        0.8
    }
}
