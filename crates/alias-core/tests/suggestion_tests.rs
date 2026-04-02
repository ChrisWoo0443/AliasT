use alias_core::suggest;

#[test]
fn suggest_git_checkout() {
    let result = suggest("git ch");
    assert_eq!(result, Some("eckout main".to_string()));
}

#[test]
fn suggest_empty_input_returns_none() {
    let result = suggest("");
    assert_eq!(result, None);
}

#[test]
fn suggest_ls_returns_dash_la() {
    let result = suggest("ls");
    assert_eq!(result, Some(" -la".to_string()));
}

#[test]
fn suggest_unknown_command_returns_none() {
    let result = suggest("unknown_cmd_xyz");
    assert_eq!(result, None);
}

#[test]
fn suggest_git_commit() {
    let result = suggest("git co");
    assert_eq!(result, Some("mmit -m \"\"".to_string()));
}

#[test]
fn suggest_cd_returns_dotdot() {
    let result = suggest("cd ");
    assert_eq!(result, Some("..".to_string()));
}
