use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    config::{ConfigAppearance, ConfigFiles, defaults::FileDefaults, types::CfgPath},
    utils::files::home_dir,
};

// With Root Implementations
pub trait WithRoot {
    fn with_root(root: &Path) -> Self;
}
impl WithRoot for ConfigAppearance {
    fn with_root(root: &Path) -> Self {
        let mut root = root.to_path_buf();
        if root.ends_with("/") {
            root.pop();
        }
        let root = root.to_str();
        fn use_root(root: Option<&str>, path: PathBuf) -> Option<PathBuf> {
            let root = root?;
            let home = home_dir().ok()?;
            let base = home.join(".config/sherlock");

            if let Ok(suffix) = path.strip_prefix(&base) {
                Some(Path::new(root).join(suffix))
            } else {
                None
            }
        }
        let icon_paths: Vec<CfgPath> = FileDefaults::icon_paths()
            .into_iter()
            .filter_map(|s| use_root(root, s.to_path_buf()))
            .map(CfgPath::from)
            .collect();

        Self {
            icon_paths,
            ..Default::default()
        }
    }
}

impl WithRoot for ConfigFiles {
    fn with_root(root: &Path) -> Self {
        let mut root = root.to_path_buf();
        if root.ends_with("/") {
            root.pop();
        }
        fn use_root(root: &Path, path: Arc<Path>) -> Arc<Path> {
            if let Ok(stripped) = path.strip_prefix("~/.config/sherlock") {
                root.join(stripped).into()
            } else {
                path
            }
        }

        Self {
            config: use_root(&root, FileDefaults::config()).into(),
            fallback: use_root(&root, FileDefaults::fallback()).into(),
            alias: use_root(&root, FileDefaults::alias()).into(),
            ignore: use_root(&root, FileDefaults::ignore()).into(),
            actions: use_root(&root, FileDefaults::actions()).into(),
        }
    }
}
