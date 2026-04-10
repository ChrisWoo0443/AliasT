use aliast_daemon::doctor::{
    check_ai_backend_configured_with, check_api_key_present_with, check_daemon_running,
    check_daemon_running_at, check_history_db_at,
};
use std::io::Write;

#[test]
fn test_check_daemon_running_returns_valid_check() {
    let check = check_daemon_running();
    assert_eq!(check.name, "Daemon running");
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
    use std::path::PathBuf;
    let bogus_path = PathBuf::from("/tmp/aliast-nonexistent-test/aliast.sock");
    let check = check_daemon_running_at(&bogus_path);
    assert!(!check.passed, "should fail when socket does not exist");
    assert!(check.fix.is_some(), "should provide fix instruction");
    assert!(check.fix.unwrap().contains("aliast start"));
}

#[test]
fn test_check_ai_backend_configured_with_model() {
    let check = check_ai_backend_configured_with("ollama", Some("llama3.2"));
    assert!(check.passed, "should pass when model is set");
    assert!(check.fix.is_none());
    assert!(check.detail.contains("llama3.2"));
}

#[test]
fn test_check_ai_backend_configured_without_model() {
    let check = check_ai_backend_configured_with("ollama", None);
    assert!(!check.passed, "should fail when no model set");
    assert!(check.fix.is_some());
    assert!(check.fix.unwrap().contains("ALIAST_NL_MODEL"));
}

#[test]
fn test_check_ai_backend_configured_empty_backend() {
    let check = check_ai_backend_configured_with("", Some("llama3.2"));
    assert!(check.passed);
    assert!(check.detail.contains("ollama (default)"));
}

#[test]
fn test_check_history_db_exists() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("history.db");
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
    assert!(check.passed, "missing DB is OK -- will be created on first start");
    assert!(check.detail.contains("will be created"));
}

#[test]
fn test_check_api_key_present_ollama() {
    let check = check_api_key_present_with("ollama", false, false);
    assert!(check.passed, "ollama needs no API key");
}

#[test]
fn test_check_api_key_present_claude_without_key() {
    let check = check_api_key_present_with("claude", false, false);
    assert!(!check.passed, "claude backend needs API key");
    assert!(check.fix.is_some());
}

#[test]
fn test_check_api_key_present_claude_with_key() {
    let check = check_api_key_present_with("claude", true, false);
    assert!(check.passed, "claude with key should pass");
}

#[test]
fn test_check_api_key_present_openai_without_key() {
    let check = check_api_key_present_with("openai", false, false);
    assert!(!check.passed, "openai backend needs API key");
    assert!(check.fix.is_some());
}

#[test]
fn test_check_api_key_present_openai_with_key() {
    let check = check_api_key_present_with("openai", false, true);
    assert!(check.passed, "openai with key should pass");
}

#[test]
fn test_doctor_check_fix_none_when_passed() {
    let check = check_api_key_present_with("ollama", false, false);
    assert!(check.passed);
    assert!(check.fix.is_none(), "fix should be None when check passes");
}
