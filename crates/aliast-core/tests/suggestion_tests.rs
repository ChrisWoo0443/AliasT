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
fn fully_typed_frecency_winner_does_not_suggest_rarer_recent_extension() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let store = HistoryStore::open(&db_path).unwrap();

    // "git push" used heavily -- it is the frecency winner for the prefix.
    for t in 0..100 {
        store
            .record_command("git push", 1000 + t, "/proj", Some(0))
            .unwrap();
    }
    // "git pushover" used once, but more recently.
    store
        .record_command("git pushover", 100_000, "/proj", Some(0))
        .unwrap();

    // Typing the full, heavily-used command must not fall back to raw recency and
    // suggest the rarely-used longer command.
    let context = SuggestionContext::default();
    let result = suggest(&store, "git push", &context);
    assert_eq!(result, None, "got {result:?}");
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn frequently_used_older_command_outranks_a_rare_recent_one() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();
    let now = now_secs();

    // "deploy prod" used 50 times ~2 days ago.
    for i in 0..50 {
        store
            .record_command("deploy prod", now - 172_800 + i, "/proj", Some(0))
            .unwrap();
    }
    // "deploy staging" used once, 5 minutes ago.
    store
        .record_command("deploy staging", now - 300, "/proj", Some(0))
        .unwrap();

    let context = SuggestionContext::default();
    assert_eq!(
        suggest(&store, "deploy ", &context),
        Some("prod".to_string()),
        "a heavily-used command should beat a rarely-used more-recent one"
    );
}

#[test]
fn consistently_failing_command_is_demoted_below_a_working_one() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();
    let now = now_secs();

    // "build fast" used 20 times recently -- but always fails.
    for i in 0..20 {
        store
            .record_command("build fast", now - 100 + i, "/proj", Some(1))
            .unwrap();
    }
    // "build ok" used only 3 times recently -- always succeeds.
    for i in 0..3 {
        store
            .record_command("build ok", now - 100 + i, "/proj", Some(0))
            .unwrap();
    }

    let context = SuggestionContext::default();
    assert_eq!(
        suggest(&store, "build ", &context),
        Some("ok".to_string()),
        "a consistently-failing command should not outrank a working one"
    );
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
        store
            .record_command("git checkout main", now - 100 + i, "/proj", Some(0))
            .unwrap();
    }

    let context = SuggestionContext {
        cwd: Some("/proj".to_string()),
        ..Default::default()
    };
    let result = suggest(&store, "git ch", &context);
    assert_eq!(result, Some("eckout main".to_string()));
}
