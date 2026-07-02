//! `aliast import` re-imports zsh history on demand (dedup-safe) and
//! `aliast stats` summarizes the local history db.

use std::process::Command;

#[test]
fn import_then_stats_reports_history() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_aliast");
    std::fs::write(
        tmp.path().join(".zsh_history"),
        "git status\ngit status\nls -la\n",
    )
    .unwrap();

    let import = Command::new(bin)
        .arg("import")
        .env("HOME", tmp.path())
        .output()
        .unwrap();
    assert!(import.status.success(), "import failed: {import:?}");
    let out = String::from_utf8_lossy(&import.stdout);
    assert!(out.contains("3"), "should report 3 imported entries: {out}");

    // Second import is a no-op (dedup), not a duplication.
    let again = Command::new(bin)
        .arg("import")
        .env("HOME", tmp.path())
        .output()
        .unwrap();
    let out_again = String::from_utf8_lossy(&again.stdout);
    assert!(
        out_again.contains("0"),
        "re-import should add 0 new entries: {out_again}"
    );

    let stats = Command::new(bin)
        .arg("stats")
        .env("HOME", tmp.path())
        .output()
        .unwrap();
    assert!(stats.status.success(), "stats failed: {stats:?}");
    let stats_out = String::from_utf8_lossy(&stats.stdout);
    assert!(
        stats_out.contains("git status"),
        "stats should list top commands: {stats_out}"
    );
}
