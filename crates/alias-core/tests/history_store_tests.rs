use alias_core::history::{HistoryEntry, HistoryStore};

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

    store.record_command("git status", 1000, "/home").unwrap();

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
        .record_command("git checkout main", 1000, "/home")
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
        .record_command("git checkout develop", 1000, "/home")
        .unwrap();
    store
        .record_command("git checkout main", 2000, "/home")
        .unwrap();

    let result = store.suggest_prefix("git ch").unwrap();
    assert_eq!(result, Some("git checkout main".to_string()));
}

#[test]
fn suggest_prefix_is_case_sensitive() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("git status", 1000, "/home").unwrap();

    let result = store.suggest_prefix("Git").unwrap();
    assert_eq!(result, None);
}

#[test]
fn suggest_prefix_escapes_sql_wildcards() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("something else", 1000, "/home")
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
