use super::{HistoryEntry, SuggestionContext};
use rusqlite::Connection;
use std::path::Path;

/// SQLite-backed command history store for prefix-based suggestion lookups.
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// Opens (or creates) a SQLite database at `path` with WAL mode and
    /// case-sensitive LIKE enabled, creating the history table and index if needed.
    /// Runs schema migrations to ensure exit_code column exists.
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
                cwd       TEXT NOT NULL DEFAULT '',
                exit_code INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_history_cmd_ts
                ON history (command, timestamp DESC);",
        )?;

        // Schema migration: add exit_code column if missing (user_version tracking)
        let user_version: i32 =
            conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if user_version < 1 {
            // Check if exit_code column already exists (table may have been created fresh above)
            let has_exit_code = conn
                .prepare("SELECT exit_code FROM history LIMIT 0")
                .is_ok();

            if !has_exit_code {
                conn.execute_batch(
                    "ALTER TABLE history ADD COLUMN exit_code INTEGER;",
                )?;
            }

            conn.execute_batch("PRAGMA user_version = 1;")?;
        }

        Ok(Self { conn })
    }

    /// Records a single command execution into the history store.
    pub fn record_command(
        &self,
        command: &str,
        timestamp: i64,
        cwd: &str,
        exit_code: Option<i32>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO history (command, timestamp, cwd, exit_code) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![command, timestamp, cwd, exit_code],
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

    /// Returns the top frecency-ranked command matching the given prefix,
    /// considering recency, frequency, directory affinity, and exit code.
    ///
    /// Scoring:
    /// - Recency: last hour=100, today=80, this week=60, this month=40, older=20
    /// - Frequency: >=50 uses=30, >=10=20, >=5=15, >=2=10, else=5
    /// - Directory bonus: +20 if any execution was in context.cwd
    /// - Exit code penalty: -15 if majority of executions failed
    pub fn suggest_ranked(
        &self,
        prefix: &str,
        context: &SuggestionContext,
    ) -> Result<Option<String>, rusqlite::Error> {
        if prefix.is_empty() {
            return Ok(None);
        }

        let escaped_prefix = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let like_pattern = format!("{}%", escaped_prefix);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let context_cwd = context.cwd.as_deref().unwrap_or("");

        let mut statement = self.conn.prepare_cached(
            "SELECT command,
                    -- Recency score based on most recent execution
                    CASE
                        WHEN MAX(timestamp) >= (:now - 3600) THEN 100
                        WHEN MAX(timestamp) >= (:now - 86400) THEN 80
                        WHEN MAX(timestamp) >= (:now - 604800) THEN 60
                        WHEN MAX(timestamp) >= (:now - 2592000) THEN 40
                        ELSE 20
                    END AS recency_score,
                    -- Frequency score
                    CASE
                        WHEN COUNT(*) >= 50 THEN 30
                        WHEN COUNT(*) >= 10 THEN 20
                        WHEN COUNT(*) >= 5 THEN 15
                        WHEN COUNT(*) >= 2 THEN 10
                        ELSE 5
                    END AS frequency_score,
                    -- Directory bonus: +20 if any execution was in context cwd
                    CASE
                        WHEN :cwd != '' AND SUM(CASE WHEN cwd = :cwd THEN 1 ELSE 0 END) > 0 THEN 20
                        ELSE 0
                    END AS directory_bonus,
                    -- Exit code penalty: -15 if majority of executions failed
                    CASE
                        WHEN SUM(CASE WHEN exit_code IS NOT NULL AND exit_code != 0 THEN 1 ELSE 0 END) * 2 > COUNT(CASE WHEN exit_code IS NOT NULL THEN 1 END) THEN -15
                        ELSE 0
                    END AS exit_penalty
             FROM history
             WHERE command LIKE :pattern ESCAPE '\\'
             GROUP BY command
             ORDER BY (recency_score + frequency_score + directory_bonus + exit_penalty) DESC,
                      MAX(timestamp) DESC
             LIMIT 1",
        )?;

        let result = statement.query_row(
            rusqlite::named_params! {
                ":now": now,
                ":cwd": context_cwd,
                ":pattern": like_pattern,
            },
            |row| row.get::<_, String>(0),
        );

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
                "INSERT INTO history (command, timestamp, cwd, exit_code) VALUES (?1, ?2, '', NULL)",
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
