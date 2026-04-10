use aliast_daemon::doctor::{
    check_ai_backend_configured, check_api_key_present, check_daemon_running, check_history_db_at,
};
use std::io::Write;

#[test]
fn test_check_daemon_running_returns_valid_check() {
    let check = check_daemon_running();
    assert_eq!(check.name, "Daemon running");
    // The daemon may or may not be running in CI/dev -- verify structure is correct
    if check.passed {
        assert!(check.fix.is_none(), "passing check should have no fix");
        assert!(check.detail.contains("Connected"));
    } else {
        assert!(check.fix.is_some(), "should provide fix instruction");
        assert!(check.fix.unwrap().contains("aliast start"));
    }
}

#[test]
fn test_check_daemon_running_at_nonexistent_socket() {
    use aliast_daemon::doctor::check_daemon_running_at;
    use std::path::PathBuf;

    let bogus_path = PathBuf::from("/tmp/aliast-nonexistent-test/aliast.sock");
    let check = check_daemon_running_at(&bogus_path);
    assert!(!check.passed, "should fail when socket does not exist");
    assert!(check.fix.is_some(), "should provide fix instruction");
    assert!(check.fix.unwrap().contains("aliast start"));
}

#[test]
fn test_check_ai_backend_configured_with_model() {
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::set_var("ALIAST_NL_MODEL", "llama3.2");
        std::env::set_var("ALIAST_NL_BACKEND", "ollama");
    }
    let check = check_ai_backend_configured();
    assert!(check.passed, "should pass when model is set");
    assert!(check.fix.is_none());
    assert!(check.detail.contains("llama3.2"));
    // Clean up
    unsafe {
        std::env::remove_var("ALIAST_NL_MODEL");
        std::env::remove_var("ALIAST_NL_BACKEND");
    }
}

#[test]
fn test_check_ai_backend_configured_without_model() {
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::remove_var("ALIAST_NL_MODEL");
    }
    let check = check_ai_backend_configured();
    assert!(!check.passed, "should fail when no model set");
    assert!(check.fix.is_some());
    assert!(check.fix.unwrap().contains("ALIAST_NL_MODEL"));
}

#[test]
fn test_check_history_db_exists() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("history.db");
    // Create a non-empty file
    let mut file = std::fs::File::create(&db_path).unwrap();
    file.write_all(b"test data").unwrap();
    let check = check_history_db_at(&db_path);
    assert!(check.passed);
    assert!(check.fix.is_none());
}

#[test]
fn test_check_history_db_missing() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("nonexistent.db");
    let check = check_history_db_at(&db_path);
    assert!(
        check.passed,
        "missing DB is OK -- will be created on first start"
    );
    assert!(check.detail.contains("will be created"));
}

#[test]
fn test_check_api_key_present_ollama() {
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::set_var("ALIAST_NL_BACKEND", "ollama");
    }
    let check = check_api_key_present();
    assert!(check.passed, "ollama needs no API key");
    unsafe {
        std::env::remove_var("ALIAST_NL_BACKEND");
    }
}

#[test]
fn test_check_api_key_present_claude_without_key() {
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::set_var("ALIAST_NL_BACKEND", "claude");
        std::env::remove_var("ALIAST_ANTHROPIC_KEY");
    }
    let check = check_api_key_present();
    assert!(!check.passed, "claude backend needs API key");
    assert!(check.fix.is_some());
    unsafe {
        std::env::remove_var("ALIAST_NL_BACKEND");
    }
}

#[test]
fn test_check_api_key_present_openai_without_key() {
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::set_var("ALIAST_NL_BACKEND", "openai");
        std::env::remove_var("ALIAST_OPENAI_KEY");
    }
    let check = check_api_key_present();
    assert!(!check.passed, "openai backend needs API key");
    assert!(check.fix.is_some());
    unsafe {
        std::env::remove_var("ALIAST_NL_BACKEND");
    }
}

#[test]
fn test_doctor_check_fix_none_when_passed() {
    // Ollama needs no key, so this should pass and have no fix
    // SAFETY: tests that manipulate env vars run serially with --test-threads=1
    unsafe {
        std::env::set_var("ALIAST_NL_BACKEND", "ollama");
    }
    let check = check_api_key_present();
    assert!(check.passed);
    assert!(check.fix.is_none(), "fix should be None when check passes");
    unsafe {
        std::env::remove_var("ALIAST_NL_BACKEND");
    }
}
