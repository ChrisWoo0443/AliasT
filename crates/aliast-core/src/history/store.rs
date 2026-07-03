use super::{HistoryEntry, SuggestionContext};
use rusqlite::Connection;
use std::path::Path;

/// Maximum number of history rows to retain. Bounds disk and query growth while
/// preserving enough history for meaningful frecency ranking. Enforced at open().
const MAX_HISTORY_ENTRIES: i64 = 100_000;

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

        // Restrict the database and its WAL/SHM sidecars to the owner: shell
        // history routinely contains secrets and must not be world-readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for suffix in ["", "-wal", "-shm"] {
                let mut os_path = path.as_os_str().to_os_string();
                os_path.push(suffix);
                let sidecar = std::path::PathBuf::from(os_path);
                if sidecar.exists() {
                    let _ =
                        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o600));
                }
            }
        }

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
        let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if user_version < 1 {
            // Check if exit_code column already exists (table may have been created fresh above)
            let has_exit_code = conn
                .prepare("SELECT exit_code FROM history LIMIT 0")
                .is_ok();

            if !has_exit_code {
                conn.execute_batch("ALTER TABLE history ADD COLUMN exit_code INTEGER;")?;
            }

            conn.execute_batch("PRAGMA user_version = 1;")?;
        }

        if user_version < 2 {
            // Acceptance feedback: one row per command the user has accepted a
            // ghost suggestion for. Feeds a ranking bonus.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS acceptances (
                    command  TEXT PRIMARY KEY,
                    count    INTEGER NOT NULL DEFAULT 0,
                    last_ts  INTEGER NOT NULL DEFAULT 0
                 );
                 PRAGMA user_version = 2;",
            )?;
        }

        let store = Self { conn };
        store.prune(MAX_HISTORY_ENTRIES)?;
        Ok(store)
    }

    /// Deletes all but the most recent `max_entries` rows (by insertion order),
    /// bounding unbounded history growth.
    pub fn prune(&self, max_entries: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM history
             WHERE id NOT IN (
                 SELECT id FROM history ORDER BY id DESC LIMIT ?1
             )",
            rusqlite::params![max_entries],
        )?;
        Ok(())
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

    /// Records that the user accepted a ghost suggestion for `command`.
    /// The signal feeds a ranking bonus in [`suggest_ranked_at`](Self::suggest_ranked_at).
    pub fn record_acceptance(&self, command: &str) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO acceptances (command, count, last_ts) VALUES (?1, 1, ?2)
             ON CONFLICT(command) DO UPDATE SET count = count + 1, last_ts = ?2",
            rusqlite::params![command, now],
        )?;
        Ok(())
    }

    /// Returns the full command text of the most recent history entry matching
    /// the given prefix, or None if no match exists.
    ///
    /// Uses case-sensitive LIKE with escaped SQL wildcards for safe prefix matching.
    pub fn suggest_prefix(&self, prefix: &str) -> Result<Option<String>, rusqlite::Error> {
        // Escape SQL LIKE wildcards in the prefix
        let escaped_prefix = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
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
    /// Scoring (frequency is weighted to stay competitive with recency, and the
    /// failure penalty is strong enough to actually demote a failing command):
    /// - Recency: last hour=100, today=80, this week=60, this month=40, older=20
    /// - Frequency: >=50 uses=50, >=10=35, >=5=25, >=2=15, else=5
    /// - Directory bonus: +20 if any execution was in context.cwd
    /// - Exit code penalty: -40 if majority of executions failed
    pub fn suggest_ranked(
        &self,
        prefix: &str,
        context: &SuggestionContext,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.suggest_ranked_at(prefix, context, 0)
    }

    /// Like [`suggest_ranked`](Self::suggest_ranked), but returns the candidate
    /// at rank `skip` (0 = best). Powers fish-style suggestion cycling.
    pub fn suggest_ranked_at(
        &self,
        prefix: &str,
        context: &SuggestionContext,
        skip: u32,
    ) -> Result<Option<String>, rusqlite::Error> {
        Ok(self
            .suggest_ranked_list(prefix, context, skip + 1)?
            .into_iter()
            .nth(skip as usize))
    }

    /// The first `limit` frecency-ranked full commands matching `prefix`.
    /// Powers the merged multi-source suggestion pipeline, where cycling
    /// indexes one concatenated candidate list.
    pub fn suggest_ranked_list(
        &self,
        prefix: &str,
        context: &SuggestionContext,
        limit: u32,
    ) -> Result<Vec<String>, rusqlite::Error> {
        if prefix.is_empty() {
            return Ok(Vec::new());
        }

        let escaped_prefix = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like_pattern = format!("{}%", escaped_prefix);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let context_cwd = context.cwd.as_deref().unwrap_or("");

        let mut statement = self.conn.prepare_cached(
            "SELECT history.command,
                    CASE
                        WHEN MAX(timestamp) >= (:now - 3600) THEN 100
                        WHEN MAX(timestamp) >= (:now - 86400) THEN 80
                        WHEN MAX(timestamp) >= (:now - 604800) THEN 60
                        WHEN MAX(timestamp) >= (:now - 2592000) THEN 40
                        ELSE 20
                    END AS recency_score,
                    CASE
                        WHEN COUNT(*) >= 50 THEN 50
                        WHEN COUNT(*) >= 10 THEN 35
                        WHEN COUNT(*) >= 5 THEN 25
                        WHEN COUNT(*) >= 2 THEN 15
                        ELSE 5
                    END AS frequency_score,
                    CASE
                        WHEN :cwd != '' AND SUM(CASE WHEN cwd = :cwd THEN 1 ELSE 0 END) > 0 THEN 20
                        ELSE 0
                    END AS directory_bonus,
                    -- Exit code penalty: -40 if majority of executions failed
                    CASE
                        WHEN SUM(CASE WHEN exit_code IS NOT NULL AND exit_code != 0 THEN 1 ELSE 0 END) * 2 > COUNT(CASE WHEN exit_code IS NOT NULL THEN 1 END) THEN -40
                        ELSE 0
                    END AS exit_penalty,
                    CASE
                        WHEN COALESCE(MAX(acceptances.count), 0) >= 5 THEN 25
                        WHEN COALESCE(MAX(acceptances.count), 0) >= 1 THEN 15
                        ELSE 0
                    END AS acceptance_bonus
             FROM history
             LEFT JOIN acceptances ON acceptances.command = history.command
             WHERE history.command LIKE :pattern ESCAPE '\\'
             GROUP BY history.command
             ORDER BY (recency_score + frequency_score + directory_bonus + exit_penalty + acceptance_bonus) DESC,
                      MAX(timestamp) DESC
             LIMIT :limit",
        )?;

        let rows = statement.query_map(
            rusqlite::named_params! {
                ":now": now,
                ":cwd": context_cwd,
                ":pattern": like_pattern,
                ":limit": limit,
            },
            |row| row.get::<_, String>(0),
        )?;
        rows.collect()
    }

    /// Distinct `cd ...` commands recorded while in `cwd`, most-used first.
    /// Feeds navigation-history ranking for directory completion.
    pub fn cd_commands_for_cwd(
        &self,
        cwd: &str,
        limit: u32,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let mut statement = self.conn.prepare_cached(
            "SELECT command FROM history
             WHERE cwd = ?1 AND command LIKE 'cd %'
             GROUP BY command
             ORDER BY COUNT(*) DESC, MAX(timestamp) DESC
             LIMIT ?2",
        )?;
        let rows =
            statement.query_map(rusqlite::params![cwd, limit], |row| row.get::<_, String>(0))?;
        rows.collect()
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

    /// Imports history entries, skipping any (command, timestamp) pair already
    /// present -- safe to run repeatedly (`aliast import`). Returns the number
    /// of newly inserted rows.
    pub fn import_entries_dedup(&self, entries: &[HistoryEntry]) -> Result<usize, rusqlite::Error> {
        let transaction = self.conn.unchecked_transaction()?;

        let mut inserted = 0;
        for entry in entries {
            let timestamp = entry.timestamp.unwrap_or(0);
            let changed = transaction.execute(
                "INSERT INTO history (command, timestamp, cwd, exit_code)
                 SELECT ?1, ?2, '', NULL
                 WHERE NOT EXISTS (
                     SELECT 1 FROM history WHERE command = ?1 AND timestamp = ?2
                 )",
                rusqlite::params![entry.command, timestamp],
            )?;
            inserted += changed;
        }

        transaction.commit()?;
        Ok(inserted)
    }

    /// Returns the most-used commands with their usage counts, best first.
    pub fn top_commands(&self, limit: u32) -> Result<Vec<(String, i64)>, rusqlite::Error> {
        let mut statement = self.conn.prepare_cached(
            "SELECT command, COUNT(*) AS uses FROM history
             GROUP BY command ORDER BY uses DESC, MAX(timestamp) DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(rusqlite::params![limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect()
    }

    /// Returns the most-accepted suggestions with their counts, best first.
    pub fn top_accepted(&self, limit: u32) -> Result<Vec<(String, i64)>, rusqlite::Error> {
        let mut statement = self.conn.prepare_cached(
            "SELECT command, count FROM acceptances ORDER BY count DESC, last_ts DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(rusqlite::params![limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect()
    }

    /// Returns the total number of entries in the history store.
    pub fn count(&self) -> Result<i64, rusqlite::Error> {
        self.conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
    }
}
