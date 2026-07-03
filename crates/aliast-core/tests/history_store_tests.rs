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

    store
        .record_command("git status", 1000, "/home", None)
        .unwrap();

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

    store
        .record_command("git status", 1000, "/home", None)
        .unwrap();

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
fn fresh_db_has_current_user_version_and_exit_code_column() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let _store = HistoryStore::open(&db_path).unwrap();

    // Verify by opening raw connection
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);

    // Verify exit_code column exists
    let result = conn.execute(
        "INSERT INTO history (command, timestamp, cwd, exit_code) VALUES ('test', 0, '', 0)",
        [],
    );
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
        )
        .unwrap();
        conn.execute(
            "INSERT INTO history (command, timestamp, cwd) VALUES ('old_cmd', 500, '/old')",
            [],
        )
        .unwrap();
    }

    // Open with HistoryStore should migrate
    let store = HistoryStore::open(&db_path).unwrap();

    // Old data still accessible
    let count = store.count().unwrap();
    assert_eq!(count, 1);

    // Can record with exit_code now
    store
        .record_command("new_cmd", 1000, "/home", Some(0))
        .unwrap();
    assert_eq!(store.count().unwrap(), 2);

    // Verify migrations ran to the current schema version
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
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

    store
        .record_command("bad_cmd", 1000, "/home", Some(127))
        .unwrap();
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
        store
            .record_command("git status", now - 100 + i, "/home", Some(0))
            .unwrap();
    }
    // Record "git stash" 1 time
    store
        .record_command("git stash", now - 50, "/home", Some(0))
        .unwrap();

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
    store
        .record_command("git stash", now - 2_000_000, "/home", Some(0))
        .unwrap();
    // "git status" was run recently
    store
        .record_command("git status", now - 10, "/home", Some(0))
        .unwrap();

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
    store
        .record_command("git stash", now - 10, "/proj", Some(0))
        .unwrap();
    // "git status" in /other -- run once recently
    store
        .record_command("git status", now - 10, "/other", Some(0))
        .unwrap();

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
        store
            .record_command("git stash", now - 10 + i, "/home", Some(1))
            .unwrap();
    }
    // "git status" always succeeds
    for i in 0..3 {
        store
            .record_command("git status", now - 10 + i, "/home", Some(0))
            .unwrap();
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

    store
        .record_command("git stash", now - 10, "/home", Some(1))
        .unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("git st", &context).unwrap();
    assert_eq!(result, Some("git stash".to_string()));
}

#[test]
fn frecency_empty_prefix_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git status", 1000, "/home", Some(0))
        .unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("", &context).unwrap();
    assert_eq!(result, None);
}

#[test]
fn frecency_no_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git status", 1000, "/home", Some(0))
        .unwrap();

    let context = SuggestionContext::default();
    let result = store.suggest_ranked("cargo ", &context).unwrap();
    assert_eq!(result, None);
}

#[test]
fn record_acceptance_boosts_ranking() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Two equally-frequent, equally-recent candidates.
    for i in 0..3 {
        store
            .record_command("make alpha", now - 50 + i, "/p", Some(0))
            .unwrap();
        store
            .record_command("make beta", now - 50 + i, "/p", Some(0))
            .unwrap();
    }

    // The user repeatedly accepts the beta suggestion.
    for _ in 0..3 {
        store.record_acceptance("make beta").unwrap();
    }

    let context = SuggestionContext::default();
    let top = store.suggest_ranked("make ", &context).unwrap();
    assert_eq!(
        top,
        Some("make beta".to_string()),
        "accepted suggestions should outrank otherwise-equal candidates"
    );
}

#[test]
fn record_acceptance_survives_reopen() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    {
        let store = HistoryStore::open(&db_path).unwrap();
        store.record_acceptance("git status").unwrap();
    }
    let store = HistoryStore::open(&db_path).unwrap();
    // Recording again must upsert, not fail on a unique constraint.
    store.record_acceptance("git status").unwrap();
}

#[test]
fn import_entries_dedup_skips_existing_rows() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();

    let entries = vec![
        HistoryEntry {
            command: "git status".to_string(),
            timestamp: Some(1000),
        },
        HistoryEntry {
            command: "ls -la".to_string(),
            timestamp: Some(2000),
        },
    ];
    assert_eq!(store.import_entries_dedup(&entries).unwrap(), 2);

    // Re-importing the same file must not duplicate rows...
    assert_eq!(store.import_entries_dedup(&entries).unwrap(), 0);
    assert_eq!(store.count().unwrap(), 2);

    // ...while genuinely new entries still land.
    let more = vec![HistoryEntry {
        command: "cargo build".to_string(),
        timestamp: Some(3000),
    }];
    assert_eq!(store.import_entries_dedup(&more).unwrap(), 1);
    assert_eq!(store.count().unwrap(), 3);
}

#[test]
fn top_commands_orders_by_usage() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();

    for i in 0..5 {
        store
            .record_command("git status", 100 + i, "/p", None)
            .unwrap();
    }
    for i in 0..2 {
        store.record_command("ls", 200 + i, "/p", None).unwrap();
    }

    let top = store.top_commands(10).unwrap();
    assert_eq!(top[0], ("git status".to_string(), 5));
    assert_eq!(top[1], ("ls".to_string(), 2));

    let capped = store.top_commands(1).unwrap();
    assert_eq!(capped.len(), 1);
}

#[test]
fn suggest_ranked_list_returns_ordered_candidates() {
    let tmp = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp.path().join("t.db")).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    for i in 0..10 {
        store
            .record_command("git push", now - 50 + i, "/p", Some(0))
            .unwrap();
    }
    for i in 0..3 {
        store
            .record_command("git pull", now - 50 + i, "/p", Some(0))
            .unwrap();
    }
    store
        .record_command("git pack-refs", now - 50, "/p", Some(0))
        .unwrap();

    let context = SuggestionContext::default();
    let list = store.suggest_ranked_list("git p", &context, 8).unwrap();
    assert_eq!(list, vec!["git push", "git pull", "git pack-refs"]);

    let limited = store.suggest_ranked_list("git p", &context, 2).unwrap();
    assert_eq!(limited, vec!["git push", "git pull"]);
}

#[test]
fn suggest_ranked_list_empty_prefix_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp.path().join("t.db")).unwrap();
    assert!(
        store
            .suggest_ranked_list("", &SuggestionContext::default(), 8)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn cd_commands_for_cwd_ranks_by_frequency_within_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp.path().join("t.db")).unwrap();

    for t in 0..5 {
        store
            .record_command("cd crates", 1000 + t, "/repo", Some(0))
            .unwrap();
    }
    store
        .record_command("cd plugin", 2000, "/repo", Some(0))
        .unwrap();
    // Same command from a DIFFERENT directory must not count.
    for t in 0..9 {
        store
            .record_command("cd elsewhere", 3000 + t, "/other", Some(0))
            .unwrap();
    }
    // Non-cd commands in /repo must not appear.
    store
        .record_command("ls -la", 4000, "/repo", Some(0))
        .unwrap();

    let got = store.cd_commands_for_cwd("/repo", 32).unwrap();
    assert_eq!(got, vec!["cd crates", "cd plugin"]);
}
