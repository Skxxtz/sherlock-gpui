use std::sync::Arc;

use gpui::{App, SharedString, Task};
use tokio::sync::mpsc;
use utils::{ProcessResult, ResultHeap};

use crate::{
    app::LauncherWeakEntity,
    features::process::ProcessLauncher,
    launcher::{Children, LauncherConfig, variant_type::LauncherType},
    ui::{
        views::{
            process::{backends::ProcessBackend, view::ProcessView},
            query::ResultsSink,
        },
        widgets::{RenderableChild, process::ProcessData},
    },
};

mod backends;
mod utils;
pub mod view;

#[derive(Default)]
pub struct ProcessModel {
    backend: ProcessBackend,
    launcher: Arc<LauncherConfig>,
    results: Vec<ProcessResult>,
    cancel_tx: Option<mpsc::Sender<()>>,
    _poll_task: Option<Task<()>>,
}

impl ProcessModel {
    pub fn new(launcher: Arc<LauncherConfig>) -> Self {
        if let LauncherType::Process(ProcessLauncher { max_results }) = launcher.launcher_type {
            Self {
                launcher,
                results: Vec::with_capacity(max_results),
                ..Default::default()
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
        sink: ResultsSink<ProcessView>,
        cx: &mut App,
    ) {
        self.cancel_tx = None;
        self._poll_task = None;
        self.results.clear();

        let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
        self.cancel_tx = Some(cancel_tx);

        let backend = self.backend.clone();
        let cap = self.results.capacity();
        let launcher = Arc::clone(&self.launcher);
        let id = launcher.id();

        let poll_task = cx.spawn(async move |cx| {
            let results = cx
                .background_executor()
                .spawn({
                    let query = query_lower.clone();
                    async move {
                        let mut heap = ResultHeap::new(cap);
                        backend.search(query, &mut heap, cancel_rx);
                        heap.snapshot()
                    }
                })
                .await;

            let children: Children = results
                .into_iter()
                .map(|r| {
                    RenderableChild::new(
                        launcher.clone(),
                        ProcessData::new(r.name.clone(), r.pid, r.ppid)
                            .with_icon_name("sherlock-process"),
                    )
                })
                .collect();
            let indices = children.ids().map(|c| (id, c)).collect();
            let query: SharedString = query_lower.as_ref().to_owned().into();
            cx.update(|cx| {
                if let Some(entity) = result_entity.upgrade() {
                    entity.update(cx, |e, _| {
                        if let Some(launcher) = e.make_mut().get_mut(&id) {
                            launcher.children = children;
                        }
                    });
                }
                sink.deliver(indices, query, cx);
            });
        });

        self._poll_task = Some(poll_task);
    }
}
