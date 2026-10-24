use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::ui::model::{
    file::{
        FileSearchUtility, MAX_SEARCH_DEPTH,
        backends::FileSearchProvider,
        filter::FileFilter,
        utils::{FileResult, ResultHeap},
    },
    utils::CiUtils,
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WalkdirBackend;

impl FileSearchProvider for WalkdirBackend {
    fn name(&self) -> &'static str {
        "walkdir"
    }

    fn search(
        &self,
        query: Arc<str>,
        filter: Arc<FileFilter>,
        paths: Arc<Vec<PathBuf>>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
        tx: &mpsc::Sender<Vec<FileResult>>,
    ) -> bool {
        const BATCH: usize = 16;
        let mut files_since_send: usize = 0;

        let match_full_path = query.contains('/') || query.contains(std::path::MAIN_SEPARATOR);
        let default_files_only = filter.types.is_empty();

        for path in paths.iter() {
            let walker = walkdir::WalkDir::new(path)
                .follow_links(false)
                .min_depth(1)
                .max_depth(MAX_SEARCH_DEPTH)
                .into_iter()
                .filter_entry(|e| !FileSearchUtility::is_hidden(e))
                .filter_map(Result::ok);

            for entry in walker {
                if cancel_rx.try_recv().is_ok() || cancel_rx.is_closed() {
                    return false;
                }

                let ft = entry.file_type();
                if default_files_only && !ft.is_file() {
                    continue;
                }

                let score = if match_full_path {
                    let bytes = entry.path().as_os_str().as_encoded_bytes();
                    if !CiUtils::bytes_contain_ci(bytes, query.as_bytes()) {
                        continue;
                    }
                    FileSearchUtility::score_path(bytes, query.as_bytes())
                } else {
                    let bytes = entry.file_name().as_encoded_bytes();
                    if !CiUtils::bytes_contain_ci(bytes, query.as_bytes()) {
                        continue;
                    }
                    FileSearchUtility::score_file_ci(bytes, &query)
                };

                if !filter.matches(entry.path(), ft.is_dir(), ft.is_symlink()) {
                    continue;
                }

                if heap.push(FileResult {
                    path: entry.path().to_string_lossy().as_ref().into(),
                    score,
                }) {
                    files_since_send += 1;
                    if files_since_send >= BATCH {
                        let _ = tx.try_send(heap.snapshot());
                        files_since_send = 0;
                    }
                }
            }
        }
        true
    }
}
