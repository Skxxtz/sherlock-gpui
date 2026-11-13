use std::{
    fmt::Debug,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use bincode;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    sherlock_msg,
    utils::errors::{
        ResultExt,
        SherlockMessage,
        types::{DirAction, ErrorKind, FileAction},
    },
};

pub struct BinaryCache;

impl BinaryCache {
    const LIMIT: usize = 16 * 1024 * 1024;

    const fn config() -> impl bincode::config::Config {
        bincode::config::standard()
            .with_fixed_int_encoding()
            .with_limit::<{ Self::LIMIT }>()
    }

    pub fn encode<T: Serialize>(data: &T) -> Result<Vec<u8>, SherlockMessage> {
        bincode::serde::encode_to_vec(data, Self::config()).or_kind(ErrorKind::Serialize)
    }

    pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SherlockMessage> {
        match bincode::serde::decode_from_slice::<T, _>(bytes, Self::config()) {
            Ok((value, read)) if read == bytes.len() => Ok(value),
            Ok((_, read)) => Err(sherlock_msg!(
                Warning,
                ErrorKind::Parse(format!(
                    "{} trailing bytes after decoding",
                    bytes.len() - read
                )),
                "type mismatch"
            )),
            Err(e) => Err(sherlock_msg!(
                Warning,
                ErrorKind::Parse("binary cache".into()),
                e
            )),
        }
    }

    pub fn write<T: Serialize + Debug, P: AsRef<Path>>(
        path: P,
        data: &T,
    ) -> Result<(), SherlockMessage> {
        let cache = path.as_ref();
        let encoded = Self::encode(data)?;
        std::fs::write(cache, encoded)
            .or_kind_with(|| ErrorKind::File(FileAction::Write, cache.to_path_buf()))
    }

    pub fn read<T: DeserializeOwned + Default + Debug, P: AsRef<Path>>(
        path: P,
    ) -> Result<T, SherlockMessage> {
        let cache = path.as_ref();
        let bytes = std::fs::read(cache).map_err(|e| {
            let action = if e.kind() == io::ErrorKind::NotFound {
                FileAction::Find
            } else {
                FileAction::Read
            };
            sherlock_msg!(Warning, ErrorKind::File(action, cache.to_path_buf()), e)
        })?;
        Self::decode(&bytes).map_err(|mut e| {
            e.kind = ErrorKind::Parse(cache.to_string_lossy().to_string());
            e
        })
    }
}

pub trait JsonCache: Serialize + DeserializeOwned + Default + Clone + Debug {
    fn cache_path() -> PathBuf;

    fn write_to_cache(&self) -> Result<(), SherlockMessage> {
        let path = Self::cache_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .or_kind_with(|| ErrorKind::Dir(DirAction::Create, parent.to_path_buf()))?;
        }

        let content = simd_json::to_string(self)
            .or_kind_with(|| ErrorKind::Parse(path.to_string_lossy().to_string()))?;

        fs::write(&path, content).or_kind_with(|| ErrorKind::File(FileAction::Write, path))
    }

    fn read_from_cache(age_minutes: u64) -> Result<Option<Self>, SherlockMessage> {
        let path = Self::cache_path();

        if !path.exists() {
            return Err(sherlock_msg!(
                Warning,
                ErrorKind::File(FileAction::Find, path),
                "File not found."
            ));
        }

        let mtime = fs::metadata(&path)
            .or_kind(ErrorKind::Io)?
            .modified()
            .or_kind(ErrorKind::Io)?;

        let time_since = SystemTime::now()
            .duration_since(mtime)
            .or_kind(ErrorKind::Io)?;

        if time_since < Duration::from_secs(60 * age_minutes) {
            let reader = File::open(&path)
                .or_kind_with(|| ErrorKind::File(FileAction::Read, path.clone()))?;

            return simd_json::from_reader(reader)
                .or_kind_with(|| ErrorKind::Parse(path.to_string_lossy().to_string()))
                .map(Some);
        }

        Ok(None)
    }
}
