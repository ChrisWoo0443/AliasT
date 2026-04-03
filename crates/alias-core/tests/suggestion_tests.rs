use alias_core::history::HistoryStore;
use alias_core::suggest;

#[test]
fn suggest_returns_suffix_from_history() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout main", 1000, "/home")
        .unwrap();

    let result = suggest(&store, "git ch");
    assert_eq!(result, Some("eckout main".to_string()));
}

#[test]
fn suggest_empty_input_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let result = suggest(&store, "");
    assert_eq!(result, None);
}

#[test]
fn suggest_no_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let result = suggest(&store, "unknown_xyz");
    assert_eq!(result, None);
}

#[test]
fn suggest_exact_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout main", 1000, "/home")
        .unwrap();

    let result = suggest(&store, "git checkout main");
    assert_eq!(result, None);
}

#[test]
fn suggest_ls_returns_suffix() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("ls -la", 2000, "/home").unwrap();

    let result = suggest(&store, "ls");
    assert_eq!(result, Some(" -la".to_string()));
}
