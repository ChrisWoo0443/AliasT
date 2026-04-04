use aliast_core::history::{HistoryStore, SuggestionContext};
use aliast_core::suggest;

#[test]
fn suggest_returns_suffix_from_history() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout main", 1000, "/home", None)
        .unwrap();

    let context = SuggestionContext::default();
    let result = suggest(&store, "git ch", &context);
    assert_eq!(result, Some("eckout main".to_string()));
}

#[test]
fn suggest_empty_input_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let context = SuggestionContext::default();
    let result = suggest(&store, "", &context);
    assert_eq!(result, None);
}

#[test]
fn suggest_no_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let context = SuggestionContext::default();
    let result = suggest(&store, "unknown_xyz", &context);
    assert_eq!(result, None);
}

#[test]
fn suggest_exact_match_returns_none() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store
        .record_command("git checkout main", 1000, "/home", None)
        .unwrap();

    let context = SuggestionContext::default();
    let result = suggest(&store, "git checkout main", &context);
    assert_eq!(result, None);
}

#[test]
fn suggest_ls_returns_suffix() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    store.record_command("ls -la", 2000, "/home", None).unwrap();

    let context = SuggestionContext::default();
    let result = suggest(&store, "ls", &context);
    assert_eq!(result, Some(" -la".to_string()));
}

#[test]
fn suggest_with_context_uses_frecency_ranking() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Record "git checkout main" many times in /proj
    for i in 0..5 {
        store.record_command("git checkout main", now - 100 + i, "/proj", Some(0)).unwrap();
    }

    let context = SuggestionContext {
        cwd: Some("/proj".to_string()),
        ..Default::default()
    };
    let result = suggest(&store, "git ch", &context);
    assert_eq!(result, Some("eckout main".to_string()));
}
