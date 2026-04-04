use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use aliast_daemon::lifecycle;

#[test]
fn cleanup_removes_stale_socket_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("test.sock");

    // Create a plain file to simulate a stale socket
    std::fs::write(&socket_path, b"stale").unwrap();
    assert!(socket_path.exists());

    lifecycle::cleanup_stale_socket(&socket_path).unwrap();
    assert!(!socket_path.exists(), "stale socket file should be removed");
}

#[test]
fn cleanup_returns_error_when_daemon_is_listening() {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("active.sock");

    // Bind a real listener so connect succeeds
    let _listener = UnixListener::bind(&socket_path).unwrap();

    let result = lifecycle::cleanup_stale_socket(&socket_path);
    assert!(result.is_err(), "should return error when daemon is already running");
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("already running"),
        "error should mention 'already running', got: {error_message}"
    );
}

#[test]
fn cleanup_succeeds_when_socket_does_not_exist() {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("nonexistent.sock");

    // Should not error -- no-op when file is absent
    lifecycle::cleanup_stale_socket(&socket_path).unwrap();
}

#[test]
fn cleanup_creates_parent_directories_with_correct_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let nested_path = temp_dir.path().join("deep").join("nested").join("test.sock");

    // Parent directories do not exist yet
    assert!(!nested_path.parent().unwrap().exists());

    lifecycle::cleanup_stale_socket(&nested_path).unwrap();

    let parent = nested_path.parent().unwrap();
    assert!(parent.exists(), "parent directory should be created");

    let permissions = std::fs::metadata(parent).unwrap().permissions();
    let mode = permissions.mode() & 0o777;
    assert_eq!(mode, 0o700, "parent directory should have 0o700 permissions, got {mode:o}");
}

#[test]
fn default_socket_path_ends_with_aliast_sock() {
    let path = lifecycle::default_socket_path();
    assert!(
        path.ends_with("aliast/aliast.sock"),
        "path should end with aliast/aliast.sock, got: {}",
        path.display()
    );
}

#[test]
fn default_socket_path_uses_xdg_runtime_dir() {
    // Save original value
    let original = std::env::var("XDG_RUNTIME_DIR").ok();

    // SAFETY: Tests run serially (test thread) and we restore original value
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-xdg-runtime");
    }
    let path = lifecycle::default_socket_path();
    assert_eq!(
        path,
        PathBuf::from("/tmp/test-xdg-runtime/aliast/aliast.sock")
    );

    // Restore original value
    unsafe {
        match original {
            Some(val) => std::env::set_var("XDG_RUNTIME_DIR", val),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }
}

#[test]
fn default_socket_path_fallback_without_xdg() {
    // Save original value
    let original = std::env::var("XDG_RUNTIME_DIR").ok();

    // SAFETY: Tests run serially (test thread) and we restore original value
    unsafe {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    let path = lifecycle::default_socket_path();

    let uid = unsafe { libc::getuid() };
    let expected = PathBuf::from(format!("/tmp/aliast-{uid}/aliast/aliast.sock"));
    assert_eq!(path, expected, "should fallback to /tmp/aliast-{{uid}}/aliast/aliast.sock");

    // Restore original value
    if let Some(val) = original {
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        }
    }
}
