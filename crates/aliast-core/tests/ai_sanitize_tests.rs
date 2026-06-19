use aliast_core::ai::{AiError, sanitize_command};

#[test]
fn passes_through_plain_command() {
    assert_eq!(
        sanitize_command("find . -name '*.rs'").unwrap(),
        "find . -name '*.rs'"
    );
}

#[test]
fn strips_fenced_block_with_language_tag() {
    assert_eq!(
        sanitize_command("```bash\nfind . -name '*.rs'\n```").unwrap(),
        "find . -name '*.rs'"
    );
}

#[test]
fn strips_fenced_block_without_language() {
    assert_eq!(sanitize_command("```\nls -la\n```").unwrap(), "ls -la");
}

#[test]
fn strips_inline_backticks() {
    assert_eq!(sanitize_command("`ls -la`").unwrap(), "ls -la");
}

#[test]
fn trims_surrounding_whitespace() {
    assert_eq!(sanitize_command("  git status  ").unwrap(), "git status");
}

#[test]
fn empty_or_blank_output_is_an_error() {
    assert!(matches!(
        sanitize_command("   ").unwrap_err(),
        AiError::GenerationFailed(_)
    ));
    assert!(sanitize_command("```\n\n```").is_err());
    assert!(sanitize_command("").is_err());
}
