use aliast_daemon::connection::enrich_prompt;

#[test]
fn enrich_prompt_with_cwd_only() {
    let result = enrich_prompt("list files", Some("/proj"), None, None);
    assert!(result.contains("Current directory: /proj"));
    assert!(result.contains("list files"));
}

#[test]
fn enrich_prompt_with_failed_exit_code() {
    let result = enrich_prompt("retry command", None, Some(1), None);
    assert!(result.contains("Last command failed with exit code: 1"));
    assert!(result.contains("retry command"));
}

#[test]
fn enrich_prompt_success_exit_code_not_mentioned() {
    let result = enrich_prompt("list files", None, Some(0), None);
    assert!(!result.contains("exit code"), "success exit code should not be mentioned");
    assert_eq!(result, "list files");
}

#[test]
fn enrich_prompt_with_git_branch() {
    let result = enrich_prompt("show changes", None, None, Some("main"));
    assert!(result.contains("Git branch: main"));
    assert!(result.contains("show changes"));
}

#[test]
fn enrich_prompt_no_context_returns_original() {
    let result = enrich_prompt("list files", None, None, None);
    assert_eq!(result, "list files");
    assert!(!result.contains("[Context]"));
}

#[test]
fn enrich_prompt_with_all_context() {
    let result = enrich_prompt(
        "pull latest",
        Some("/proj"),
        Some(1),
        Some("main"),
    );
    assert!(result.contains("[Context]"));
    assert!(result.contains("Current directory: /proj"));
    assert!(result.contains("Last command failed with exit code: 1"));
    assert!(result.contains("Git branch: main"));
    assert!(result.contains("pull latest"));
}
