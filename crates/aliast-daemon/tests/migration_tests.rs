use aliast_daemon::migration::migrate_data_files;

#[test]
fn migrate_moves_files_from_old_to_new_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let old_dir = temp_dir.path().join("alias");
    let new_dir = temp_dir.path().join("aliast");

    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::write(old_dir.join("history.db"), b"test-db-content").unwrap();
    std::fs::write(old_dir.join("daemon.log"), b"test-log-content").unwrap();

    migrate_data_files(&old_dir, &new_dir).unwrap();

    assert!(!old_dir.join("history.db").exists(), "old history.db should be moved");
    assert!(!old_dir.join("daemon.log").exists(), "old daemon.log should be moved");
    assert_eq!(
        std::fs::read_to_string(new_dir.join("history.db")).unwrap(),
        "test-db-content"
    );
    assert_eq!(
        std::fs::read_to_string(new_dir.join("daemon.log")).unwrap(),
        "test-log-content"
    );
}

#[test]
fn migrate_does_not_overwrite_existing_new_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let old_dir = temp_dir.path().join("alias");
    let new_dir = temp_dir.path().join("aliast");

    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::create_dir_all(&new_dir).unwrap();
    std::fs::write(old_dir.join("history.db"), b"old-data").unwrap();
    std::fs::write(new_dir.join("history.db"), b"new-data").unwrap();

    migrate_data_files(&old_dir, &new_dir).unwrap();

    // Old file should still exist (not moved because new already exists)
    assert!(old_dir.join("history.db").exists(), "old file should remain when new exists");
    assert_eq!(
        std::fs::read_to_string(new_dir.join("history.db")).unwrap(),
        "new-data",
        "new file should not be overwritten"
    );
}

#[test]
fn migrate_no_op_when_old_files_do_not_exist() {
    let temp_dir = tempfile::tempdir().unwrap();
    let old_dir = temp_dir.path().join("alias");
    let new_dir = temp_dir.path().join("aliast");

    // Neither directory exists
    let result = migrate_data_files(&old_dir, &new_dir);
    assert!(result.is_ok(), "should succeed as no-op when old files absent");
    assert!(!new_dir.exists(), "new dir should not be created when nothing to migrate");
}

#[test]
fn migrate_handles_both_files_independently() {
    let temp_dir = tempfile::tempdir().unwrap();
    let old_dir = temp_dir.path().join("alias");
    let new_dir = temp_dir.path().join("aliast");

    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::create_dir_all(&new_dir).unwrap();

    // Only history.db in old, daemon.log already in new
    std::fs::write(old_dir.join("history.db"), b"migrate-me").unwrap();
    std::fs::write(new_dir.join("daemon.log"), b"keep-me").unwrap();

    migrate_data_files(&old_dir, &new_dir).unwrap();

    // history.db should have moved
    assert!(!old_dir.join("history.db").exists());
    assert_eq!(
        std::fs::read_to_string(new_dir.join("history.db")).unwrap(),
        "migrate-me"
    );
    // daemon.log in new should be untouched
    assert_eq!(
        std::fs::read_to_string(new_dir.join("daemon.log")).unwrap(),
        "keep-me"
    );
}
