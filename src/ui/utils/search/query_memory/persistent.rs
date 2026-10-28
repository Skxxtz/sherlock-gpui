use std::path::PathBuf;

use super::memory::{Boosts, QueryMemory, today};
use crate::{
    QUERY_MEMORY,
    sherlock_msg,
    utils::{
        cache::BinaryCache,
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        item_key::ItemKey,
    },
};

pub struct QueryMemoryGuard;
impl QueryMemoryGuard {
    pub fn popularity_count(id: &ItemKey) -> u16 {
        let Some(mem) = QUERY_MEMORY.get().and_then(|l| l.read().ok()) else {
            return 0;
        };

        let picks = mem.mem.popularity_of(id, today());
        (picks * 100.0).round().min(u16::MAX as f32) as u16
    }
}

pub struct PersistentQueryMemory {
    mem: QueryMemory,
    path: PathBuf,
}

impl PersistentQueryMemory {
    /// Load from `path`; a missing or unreadable file starts empty.
    pub fn load(path: PathBuf) -> Self {
        let mem = BinaryCache::read(&path).unwrap_or_default();
        Self { mem, path }
    }

    /// Leadn the pick and write it to disk
    pub fn record(&mut self, query: &str, key: &ItemKey) -> Result<(), SherlockMessage> {
        self.mem.record(query, key, today());
        self.save_atomic()
    }

    /// The probabilites for a given query.
    /// Vector over (ItemKey, P(Item|Query))
    pub fn boosts(&self, query: &str) -> Boosts {
        self.mem.boosts(query, today())
    }

    /// Saves the query memory to a file. Uses a temporary file to avoid corruption when Sherlock is
    /// closed or errs during the write.
    pub fn save_atomic(&self) -> Result<(), SherlockMessage> {
        let tmp = self.path.with_extension("bin.tmp");
        BinaryCache::write(&tmp, &self.mem)?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Rename, tmp),
                e
            )
        })?;
        Ok(())
    }

    pub fn import(&mut self, key: &ItemKey, count: f32, today: u32) {
        self.mem.import(key, count, today);
    }

    #[cfg(test)]
    pub fn inner(&mut self) -> &mut QueryMemory {
        &mut self.mem
    }

    #[cfg(test)]
    pub fn inner_mut(&mut self) -> &mut QueryMemory {
        &mut self.mem
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self {
            mem: QueryMemory::default(),
            path: PathBuf::new(),
        }
    }

    /// Prints a report to the terminal, showing how the probabilites for each query are
    /// distributed.
    pub fn report(&self, query: Option<&str>) {
        print!(
            "{}",
            match query {
                Some(q) => self.mem.explain(q, today()),
                None => self.mem.report(today()),
            }
        )
    }
}
