use aliast_core::history::{HistoryStore, SuggestionContext};
use aliast_core::{suggest, suggest_at};

fn empty_store() -> (tempfile::TempDir, HistoryStore) {
    let tmp = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp.path().join("t.db")).unwrap();
    (tmp, store)
}

#[test]
fn grammar_fires_when_history_is_empty() {
    let (_tmp, store) = empty_store();
    let context = SuggestionContext::default();
    assert_eq!(
        suggest(&store, "git sw", &context),
        Some("itch".to_string()),
        "git sw must complete to switch with zero history"
    );
}

#[test]
fn history_outranks_grammar() {
    let (_tmp, store) = empty_store();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    for i in 0..5 {
        store
            .record_command("git switch main", now - 10 + i, "/p", Some(0))
            .unwrap();
    }
    let context = SuggestionContext::default();
    assert_eq!(
        suggest(&store, "git sw", &context),
        Some("itch main".to_string()),
        "personalized history beats the bare grammar completion"
    );
    // ...and the bare grammar candidate is still reachable by cycling.
    assert_eq!(
        suggest_at(&store, "git sw", &context, 1),
        Some("itch".to_string())
    );
}

#[test]
fn directory_completion_flows_through_the_pipeline() {
    let (_tmp, store) = empty_store();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("projects")).unwrap();

    let context = SuggestionContext {
        cwd: Some(dir.path().to_str().unwrap().to_string()),
        ..Default::default()
    };
    assert_eq!(
        suggest(&store, "cd pro", &context),
        Some("jects/".to_string())
    );
}

#[test]
fn navigation_history_ranks_directory_candidates() {
    let (_tmp, store) = empty_store();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("alpha")).unwrap();
    std::fs::create_dir(dir.path().join("zeta")).unwrap();
    let cwd = dir.path().to_str().unwrap().to_string();

    for t in 0..5 {
        store
            .record_command("cd zeta", 1000 + t, &cwd, Some(0))
            .unwrap();
    }

    let context = SuggestionContext {
        cwd: Some(cwd),
        ..Default::default()
    };
    assert_eq!(
        suggest(&store, "cd ", &context),
        Some("zeta/".to_string()),
        "frequent navigation target must beat alphabetical order"
    );
    assert_eq!(
        suggest_at(&store, "cd ", &context, 1),
        Some("alpha/".to_string())
    );
}

#[test]
fn identical_candidates_from_two_sources_dedupe() {
    let (_tmp, store) = empty_store();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("projects")).unwrap();
    let cwd = dir.path().to_str().unwrap().to_string();

    // History already contains the exact completed command text.
    for t in 0..3 {
        store
            .record_command("cd projects/", 1000 + t, &cwd, Some(0))
            .unwrap();
    }

    let context = SuggestionContext {
        cwd: Some(cwd),
        ..Default::default()
    };
    assert_eq!(
        suggest(&store, "cd pro", &context),
        Some("jects/".to_string())
    );
    assert_eq!(
        suggest_at(&store, "cd pro", &context, 1),
        None,
        "the path candidate duplicates history and must not appear twice"
    );
}
