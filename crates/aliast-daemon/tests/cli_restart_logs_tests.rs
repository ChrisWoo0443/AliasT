//! `aliast restart` must work whether or not a daemon is running, always
//! leaving a fresh daemon serving. `aliast logs` must surface the daemon log
//! without the user memorizing its path.

use std::process::Command;
use std::time::{Duration, Instant};

fn wait_for<F: Fn() -> bool>(what: &str, timeout: Duration, cond: F) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

fn run(bin: &str, home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(bin)
        .args(args)
        .env("HOME", home)
        .output()
        .unwrap()
}

#[test]
fn restart_starts_daemon_when_none_running() {
    let tmp = tempfile::tempdir().unwrap();
    let socket = tmp.path().join("r1.sock");
    let bin = env!("CARGO_BIN_EXE_aliast");
    let socket_str = socket.to_str().unwrap();

    let out = run(bin, tmp.path(), &["restart", "--socket", socket_str]);
    assert!(out.status.success(), "restart should succeed: {:?}", out);

    wait_for("socket to appear", Duration::from_secs(5), || {
        socket.exists()
    });
    let status = run(bin, tmp.path(), &["status", "--socket", socket_str]);
    assert!(
        String::from_utf8_lossy(&status.stdout).contains("is running"),
        "daemon should be running after restart: {:?}",
        status
    );

    let _ = run(bin, tmp.path(), &["stop", "--socket", socket_str]);
    wait_for("socket removal", Duration::from_secs(5), || {
        !socket.exists()
    });
}

#[test]
fn restart_replaces_running_daemon_and_clears_autostart_marker() {
    let tmp = tempfile::tempdir().unwrap();
    let socket = tmp.path().join("r2.sock");
    let bin = env!("CARGO_BIN_EXE_aliast");
    let socket_str = socket.to_str().unwrap();

    // Bring a daemon up, then stop it -- stop leaves the autostart marker.
    let start = run(bin, tmp.path(), &["start", "--socket", socket_str]);
    assert!(start.status.success(), "start failed: {:?}", start);
    wait_for("socket to appear", Duration::from_secs(5), || {
        socket.exists()
    });
    let stop = run(bin, tmp.path(), &["stop", "--socket", socket_str]);
    assert!(stop.status.success(), "stop failed: {:?}", stop);
    wait_for("socket removal", Duration::from_secs(5), || {
        !socket.exists()
    });
    let marker = socket.parent().unwrap().join("autostart-disabled");
    assert!(marker.exists(), "stop should leave the autostart marker");

    // Restart from stopped: daemon comes up and the marker is cleared.
    let restart = run(bin, tmp.path(), &["restart", "--socket", socket_str]);
    assert!(restart.status.success(), "restart failed: {:?}", restart);
    wait_for("socket to appear", Duration::from_secs(5), || {
        socket.exists()
    });
    assert!(
        !marker.exists(),
        "restart should clear the autostart marker"
    );

    // Restart again while running: still succeeds, daemon still serving.
    let restart2 = run(bin, tmp.path(), &["restart", "--socket", socket_str]);
    assert!(
        restart2.status.success(),
        "restart over a running daemon failed: {:?}",
        restart2
    );
    wait_for("socket to reappear", Duration::from_secs(5), || {
        socket.exists()
    });
    let status = run(bin, tmp.path(), &["status", "--socket", socket_str]);
    assert!(
        String::from_utf8_lossy(&status.stdout).contains("is running"),
        "daemon should be running after second restart: {:?}",
        status
    );

    let _ = run(bin, tmp.path(), &["stop", "--socket", socket_str]);
    wait_for("socket removal", Duration::from_secs(5), || {
        !socket.exists()
    });
}

#[test]
fn logs_prints_path_and_recent_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_aliast");

    // The data dir under a fake HOME on macOS: ~/Library/Application Support/aliast
    let log_dir = tmp.path().join("Library/Application Support/aliast");
    std::fs::create_dir_all(&log_dir).unwrap();
    let log_path = log_dir.join("daemon.log");
    let lines: Vec<String> = (1..=60).map(|i| format!("log line {i}")).collect();
    std::fs::write(&log_path, lines.join("\n")).unwrap();

    let out = run(bin, tmp.path(), &["logs"]);
    assert!(out.status.success(), "logs should succeed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("daemon.log"),
        "logs should print the log path: {stdout}"
    );
    assert!(
        stdout.contains("log line 60"),
        "logs should include the newest line: {stdout}"
    );
    assert!(
        !stdout.contains("log line 1\n"),
        "logs should tail (last 50), not dump the whole file: {stdout}"
    );
}

#[test]
fn logs_with_no_entries_reports_path_and_exits_zero() {
    // Note: every aliast invocation creates (an empty) daemon.log via tracing
    // setup, so "no entries" -- not "no file" -- is the real empty state.
    let tmp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_aliast");

    let out = run(bin, tmp.path(), &["logs"]);
    assert!(out.status.success(), "logs on empty state should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no log entries yet"),
        "should explain the log is empty: {stdout}"
    );
    assert!(
        stdout.contains("daemon.log"),
        "should still print the log path: {stdout}"
    );
}
