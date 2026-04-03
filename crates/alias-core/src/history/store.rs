use super::HistoryEntry;
use rusqlite::Connection;
use std::path::Path;

/// SQLite-backed command history store for prefix-based suggestion lookups.
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// Opens (or creates) a SQLite database at `path` with WAL mode and
    /// case-sensitive LIKE enabled, creating the history table and index if needed.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA case_sensitive_like=ON;",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                command   TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT 0,
                cwd       TEXT NOT NULL DEFAULT ''
             );
             CREATE INDEX IF NOT EXISTS idx_history_cmd_ts
                ON history (command, timestamp DESC);",
        )?;

        Ok(Self { conn })
    }

    /// Records a single command execution into the history store.
    pub fn record_command(
        &self,
        command: &str,
        timestamp: i64,
        cwd: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO history (command, timestamp, cwd) VALUES (?1, ?2, ?3)",
            rusqlite::params![command, timestamp, cwd],
        )?;
        Ok(())
    }

    /// Returns the full command text of the most recent history entry matching
    /// the given prefix, or None if no match exists.
    ///
    /// Uses case-sensitive LIKE with escaped SQL wildcards for safe prefix matching.
    pub fn suggest_prefix(&self, prefix: &str) -> Result<Option<String>, rusqlite::Error> {
        // Escape SQL LIKE wildcards in the prefix
        let escaped_prefix = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let like_pattern = format!("{}%", escaped_prefix);

        let mut statement = self.conn.prepare_cached(
            "SELECT command FROM history
             WHERE command LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let result = statement.query_row(rusqlite::params![like_pattern], |row| {
            row.get::<_, String>(0)
        });

        match result {
            Ok(command) => Ok(Some(command)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Imports a batch of history entries in a single transaction.
    /// Returns the number of entries inserted.
    pub fn import_entries(&self, entries: &[HistoryEntry]) -> Result<usize, rusqlite::Error> {
        let transaction = self.conn.unchecked_transaction()?;

        let mut count = 0;
        for entry in entries {
            let timestamp = entry.timestamp.unwrap_or(0);
            transaction.execute(
                "INSERT INTO history (command, timestamp, cwd) VALUES (?1, ?2, '')",
                rusqlite::params![entry.command, timestamp],
            )?;
            count += 1;
        }

        transaction.commit()?;
        Ok(count)
    }

    /// Returns the total number of entries in the history store.
    pub fn count(&self) -> Result<i64, rusqlite::Error> {
        self.conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
    }
}
