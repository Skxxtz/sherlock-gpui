use std::path::PathBuf;

use crate::{
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

use super::memory::{Boosts, QueryMemory, today};

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
