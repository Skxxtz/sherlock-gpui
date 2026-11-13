use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

/// A path. `~` is expanded when read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CfgPath(Arc<Path>);

impl CfgPath {
    pub fn arc(&self) -> Arc<Path> {
        self.0.clone()
    }
}
impl Deref for CfgPath {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<Path> for CfgPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}
impl From<Arc<Path>> for CfgPath {
    fn from(p: Arc<Path>) -> Self {
        Self(p)
    }
}
impl From<PathBuf> for CfgPath {
    fn from(p: PathBuf) -> Self {
        Self(p.into())
    }
}
impl From<&Path> for CfgPath {
    fn from(p: &Path) -> Self {
        Self(p.into())
    }
}

impl serde::Serialize for CfgPath {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for CfgPath {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::loader::utils::deserialize_arc_path(d).map(Self)
    }
}
