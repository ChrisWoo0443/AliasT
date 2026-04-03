use super::HistoryEntry;
use rusqlite::Connection;
use std::path::Path;

/// SQLite-backed command history store for prefix-based suggestion lookups.
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// Opens (or creates) a SQLite database at `path` with WAL mode and
    /// case-sensitive LIKE enabled, creating the history table if needed.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        todo!()
    }

    /// Records a single command execution into the history store.
    pub fn record_command(
        &self,
        command: &str,
        timestamp: i64,
        cwd: &str,
    ) -> Result<(), rusqlite::Error> {
        todo!()
    }

    /// Returns the full command text of the most recent history entry matching
    /// the given prefix, or None if no match exists.
    pub fn suggest_prefix(&self, prefix: &str) -> Result<Option<String>, rusqlite::Error> {
        todo!()
    }

    /// Imports a batch of history entries in a single transaction.
    /// Returns the number of entries inserted.
    pub fn import_entries(&self, entries: &[HistoryEntry]) -> Result<usize, rusqlite::Error> {
        todo!()
    }

    /// Returns the total number of entries in the history store.
    pub fn count(&self) -> Result<i64, rusqlite::Error> {
        todo!()
    }
}
