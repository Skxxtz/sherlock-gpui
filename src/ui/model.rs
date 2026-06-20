use crate::{
    app::LauncherEntity,
    launcher::{Launcher, LauncherConfig, LauncherId, variant_type::LauncherType},
    ui::model::{file::FileSearchModel, process::ProcessModel},
};
use gpui::{App, AppContext, SharedString, Task};
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub mod emoji;
pub mod file;
pub mod home;
pub mod message;
pub mod process;
mod utils;

pub enum Model {
    Standard {
        data: LauncherEntity,
        filtered_indices: Arc<[(LauncherId, usize)]>,
        last_query: Option<SharedString>,
        deferred_render_task: Option<Task<Option<()>>>,
    },
    FileSearch {
        data: LauncherEntity,
        filtered_indices: Arc<[(LauncherId, usize)]>,
        last_query: Option<SharedString>,
        search: FileSearchModel,
    },
    Process {
        data: LauncherEntity,
        filtered_indices: Arc<[(LauncherId, usize)]>,
        last_query: Option<SharedString>,
        search: ProcessModel,
    },
}

impl Model {
    pub fn standard_with_entity(entity: LauncherEntity, cx: &mut App) -> Self {
        let range: Arc<[(LauncherId, usize)]> = entity
            .read(cx)
            .iter()
            .flat_map(|(id, launcher)| {
                (0..launcher.children.len())
                    .map(|c| (*id, c))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into();

        Self::Standard {
            data: entity,
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }
    pub fn standard(data: HashMap<LauncherId, Launcher>, cx: &mut App) -> Self {
        let range: Arc<[(LauncherId, usize)]> = data
            .iter()
            .flat_map(|(id, launcher)| {
                (0..launcher.children.len())
                    .map(|c| (*id, c))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into();

        Self::Standard {
            data: cx.new(|_| Rc::new(data)),
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }

    pub fn process(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        Self::Process {
            data: cx.new(|_| {
                Rc::new(HashMap::from([(
                    launcher.id(),
                    Launcher {
                        config: launcher.clone(),
                        children: match &launcher.launcher_type {
                            LauncherType::Process(proc) => Vec::with_capacity(proc.max_results),
                            _ => Vec::new(),
                        },
                    },
                )]))
            }),
            filtered_indices: Arc::from([]),
            last_query: None,
            search: ProcessModel::new(launcher),
        }
    }

    pub fn file_search(
        launcher: Arc<LauncherConfig>,
        dir: Option<SharedString>,
        cx: &mut App,
    ) -> Self {
        Self::FileSearch {
            data: cx.new(|_| {
                Rc::new(HashMap::from([(
                    launcher.id(),
                    Launcher {
                        config: launcher.clone(),
                        children: match &launcher.launcher_type {
                            LauncherType::Files(fs) => Vec::with_capacity(fs.max_results),
                            _ => Vec::new(),
                        },
                    },
                )]))
            }),
            filtered_indices: Arc::from([]),
            last_query: None,
            search: FileSearchModel::new(launcher, dir),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Standard {
                filtered_indices, ..
            }
            | Self::FileSearch {
                filtered_indices, ..
            }
            | Self::Process {
                filtered_indices, ..
            } => filtered_indices.len(),
        }
    }

    pub fn data(&self) -> LauncherEntity {
        match self {
            Self::Standard { data, .. }
            | Self::FileSearch { data, .. }
            | Self::Process { data, .. } => data.clone(),
        }
    }

    pub fn filtered_indices(&self) -> Arc<[(LauncherId, usize)]> {
        match self {
            Self::Standard {
                filtered_indices, ..
            }
            | Self::FileSearch {
                filtered_indices, ..
            }
            | Self::Process {
                filtered_indices, ..
            } => Arc::clone(filtered_indices),
        }
    }

    pub fn last_query(&self) -> Option<SharedString> {
        match self {
            Self::Standard { last_query, .. }
            | Self::FileSearch { last_query, .. }
            | Self::Process { last_query, .. } => last_query.clone(),
        }
    }
}
