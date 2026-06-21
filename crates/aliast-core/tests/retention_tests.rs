use aliast_core::history::HistoryStore;

#[test]
fn prune_keeps_only_the_most_recent_entries() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();

    for i in 0..10 {
        store
            .record_command(&format!("cmd{i}"), i, "/x", None)
            .unwrap();
    }

    store.prune(3).unwrap();

    assert_eq!(store.count().unwrap(), 3);
    // The three most recently inserted rows survive; the oldest are gone.
    assert!(store.suggest_prefix("cmd9").unwrap().is_some());
    assert!(store.suggest_prefix("cmd8").unwrap().is_some());
    assert!(store.suggest_prefix("cmd0").unwrap().is_none());
}

#[test]
fn prune_is_a_noop_when_under_the_cap() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::open(&tmp_dir.path().join("test.db")).unwrap();

    store.record_command("only one", 1, "/x", None).unwrap();
    store.prune(100).unwrap();

    assert_eq!(store.count().unwrap(), 1);
}
