use aliast_core::history::{HistoryEntry, HistoryStore, SuggestionContext};

#[test]
fn open_creates_db_with_table_and_index() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    // Verify table exists by querying it
    let count = store.count().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn record_command_and_query() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("git status", 1000, "/home", None).unwrap();

    let count = store.count().unwrap();
    assert_eq!(count, 1);

    let result = store.suggest_prefix("git st").unwrap();
    assert_eq!(result, Some("git status".to_string()));
}

#[test]
fn suggest_prefix_returns_suffix_match() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout main", 1000, "/home", None)
        .unwrap();

    let result = store.suggest_prefix("git ch").unwrap();
    assert_eq!(result, Some("git checkout main".to_string()));
}

#[test]
fn suggest_prefix_returns_none_when_empty() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let result = store.suggest_prefix("git ch").unwrap();
    assert_eq!(result, None);
}

#[test]
fn suggest_prefix_returns_most_recent_match() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout develop", 1000, "/home", None)
        .unwrap();
    store
        .record_command("git checkout main", 2000, "/home", None)
        .unwrap();

    let result = store.suggest_prefix("git ch").unwrap();
    assert_eq!(result, Some("git checkout main".to_string()));
}

#[test]
fn suggest_prefix_is_case_sensitive() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("git status", 1000, "/home", None).unwrap();

    let result = store.suggest_prefix("Git").unwrap();
    assert_eq!(result, None);
}

#[test]
fn suggest_prefix_escapes_sql_wildcards() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("something else", 1000, "/home", None)
        .unwrap();

    // "100%" contains SQL wildcard %, should not match everything
    let result = store.suggest_prefix("100%").unwrap();
    assert_eq!(result, None);
}

#[test]
fn import_entries_inserts_batch() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let entries = vec![
        HistoryEntry {
            command: "git status".to_string(),
            timestamp: Some(1000),
        },
        HistoryEntry {
            command: "ls -la".to_string(),
            timestamp: Some(2000),
        },
        HistoryEntry {
            command: "cargo build".to_string(),
            timestamp: None,
        },
    ];

    let count = store.import_entries(&entries).unwrap();
    assert_eq!(count, 3);
    assert_eq!(store.count().unwrap(), 3);
}

// --- Schema migration tests ---

#[test]
fn fresh_db_has_user_version_1_and_exit_code_column() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let _store = HistoryStore::open(&db_path).unwrap();

    // Verify by opening raw connection
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 1);

    // Verify exit_code column exists
    let result = conn.execute("INSERT INTO history (command, timestamp, cwd, exit_code) VALUES ('test', 0, '', 0)", []);
    assert!(result.is_ok(), "exit_code column should exist");
}

#[test]
fn migrate_existing_db_without_exit_code() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a version-0 database without exit_code column
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                command   TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT 0,
                cwd       TEXT NOT NULL DEFAULT ''
             );
             CREATE INDEX IF NOT EXISTS idx_history_cmd_ts
                ON history (command, timestamp DESC);
             PRAGMA user_version = 0;",
        ).unwrap();
        conn.execute(
            "INSERT INTO history (command, timestamp, cwd) VALUES ('old_cmd', 500, '/old')",
            [],
        ).unwrap();
    }

    // Open with HistoryStore should migrate
    let store = HistoryStore::open(&db_path).unwrap();

    // Old data still accessible
    let count = store.count().unwrap();
    assert_eq!(count, 1);

    // Can record with exit_code now
    store.record_command("new_cmd", 1000, "/home", Some(0)).unwrap();
    assert_eq!(store.count().unwrap(), 2);

    // Verify user_version is now 1
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 1);
}

// --- record_command with exit_code tests ---

#[test]
fn record_command_with_exit_code_zero() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("ls", 1000, "/home", Some(0)).unwrap();
    assert_eq!(store.count().unwrap(), 1);
}

#[test]
fn record_command_with_exit_code_nonzero() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("bad_cmd", 1000, "/home", Some(127)).unwrap();
    assert_eq!(store.count().unwrap(), 1);
}

#[test]
fn record_command_with_exit_code_none_backward_compat() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("ls", 1000, "/home", None).unwrap();
    assert_eq!(store.count().unwrap(), 1);
}

// --- Frecency ranking tests ---

#[test]
fn frecency_frequent_command_ranks_above_infrequent() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Record "git status" 10 times
    for i in 0..10 {
        store.record_command("git status", now - 100 + i, "/home", Some(0)).unwrap();
    }
    // Record "git stash" 1 time
    store.record_command("git stash", now - 50, "/home", Some(0)).unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git status".to_string()));
}

#[test]
fn frecency_recent_command_ranks_above_older() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // "git stash" was run long ago
    store.record_command("git stash", now - 2_000_000, "/home", Some(0)).unwrap();
    // "git status" was run recently
    store.record_command("git status", now - 10, "/home", Some(0)).unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git status".to_string()));
}

#[test]
fn frecency_same_directory_ranks_higher() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // "git stash" in /proj (same dir as context) -- run once recently
    store.record_command("git stash", now - 10, "/proj", Some(0)).unwrap();
    // "git status" in /other -- run once recently
    store.record_command("git status", now - 10, "/other", Some(0)).unwrap();

    let context = SuggestionContext {
        cwd: Some("/proj".to_string()),
        ..Default::default()
    };
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git stash".to_string()));
}

#[test]
fn frecency_failed_command_ranks_lower() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // "git stash" always fails
    for i in 0..3 {
        store.record_command("git stash", now - 10 + i, "/home", Some(1)).unwrap();
    }
    // "git status" always succeeds
    for i in 0..3 {
        store.record_command("git status", now - 10 + i, "/home", Some(0)).unwrap();
    }

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git status".to_string()));
}

#[test]
fn frecency_failed_command_still_returned_if_only_match() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    store.record_command("git stash", now - 10, "/home", Some(1)).unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git stash".to_string()));
}

#[test]
fn frecency_empty_prefix_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("git status", 1000, "/home", Some(0)).unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("", &context).unwrap();
    assert_eq!(result, None);
}

#[test]
fn frecency_no_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("git status", 1000, "/home", Some(0)).unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("cargo ", &context).unwrap();
    assert_eq!(result, None);
}
