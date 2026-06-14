//! The history database holds the user's shell history, which routinely contains
//! secrets. It must not be readable by other local users.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use aliast_core::history::HistoryStore;

#[test]
fn open_creates_db_file_private_to_owner() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("history.db");

    let _store = HistoryStore::open(&db_path).unwrap();

    let mode = std::fs::metadata(&db_path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o077,
        0,
        "db file must not be group/other accessible, got mode {:o}",
        mode
    );
}
