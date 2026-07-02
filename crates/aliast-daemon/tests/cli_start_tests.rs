//! `aliast start` must daemonize: return promptly with the daemon running in
//! the background, so a user's terminal is not captured.

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

#[test]
fn start_returns_promptly_and_daemon_survives_in_background() {
    let tmp = tempfile::tempdir().unwrap();
    let socket = tmp.path().join("s.sock");
    let bin = env!("CARGO_BIN_EXE_aliast");

    // `start` must exit on its own (daemonized child keeps running).
    let mut child = Command::new(bin)
        .args(["start", "--socket"])
        .arg(&socket)
        .env("HOME", tmp.path())
        .spawn()
        .unwrap();

    // The launcher process must exit promptly...
    let launcher_exit = {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!("`aliast start` did not exit -- still running in the foreground");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    assert!(launcher_exit.success(), "start should exit 0");

    // ...while the daemon (its detached child) is up and serving.
    wait_for("socket to appear", Duration::from_secs(5), || {
        socket.exists()
    });
    let out = Command::new(bin)
        .args(["status", "--socket"])
        .arg(&socket)
        .env("HOME", tmp.path())
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("is running"),
        "daemon should be running after start returns: {:?}",
        out
    );

    // Cleanup: stop must still work and the socket must disappear.
    let stop = Command::new(bin)
        .args(["stop", "--socket"])
        .arg(&socket)
        .env("HOME", tmp.path())
        .output()
        .unwrap();
    assert!(stop.status.success(), "stop failed: {:?}", stop);
    wait_for("socket removal", Duration::from_secs(5), || {
        !socket.exists()
    });
}

#[test]
fn start_foreground_flag_keeps_process_attached() {
    let tmp = tempfile::tempdir().unwrap();
    let socket = tmp.path().join("fg.sock");
    let bin = env!("CARGO_BIN_EXE_aliast");

    let mut child = Command::new(bin)
        .args(["start", "--foreground", "--socket"])
        .arg(&socket)
        .env("HOME", tmp.path())
        .spawn()
        .unwrap();

    wait_for("socket to appear", Duration::from_secs(5), || {
        socket.exists()
    });
    // With --foreground the process must still be attached (running).
    assert!(
        child.try_wait().unwrap().is_none(),
        "--foreground should keep the daemon in the foreground"
    );

    let _ = Command::new(bin)
        .args(["stop", "--socket"])
        .arg(&socket)
        .env("HOME", tmp.path())
        .output();
    let _ = child.wait();
}
