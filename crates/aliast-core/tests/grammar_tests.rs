use aliast_core::predict::grammar::complete;

#[test]
fn completes_partial_subcommand() {
    assert_eq!(complete("git sw", 8), vec!["git switch".to_string()]);
}

#[test]
fn completes_subcommand_in_table_order() {
    // git subcommands starting with "s": status, switch, stash, show (table order)
    let got = complete("git s", 8);
    assert_eq!(
        got,
        vec!["git status", "git switch", "git stash", "git show"]
    );
}

#[test]
fn empty_partial_lists_subcommands_in_table_order() {
    let got = complete("cargo ", 3);
    assert_eq!(got, vec!["cargo build", "cargo run", "cargo test"]);
}

#[test]
fn respects_limit() {
    assert_eq!(complete("git ", 2).len(), 2);
}

#[test]
fn unknown_tool_yields_nothing() {
    assert!(complete("frobnicate st", 8).is_empty());
}

#[test]
fn no_space_after_tool_yields_nothing() {
    // Completing the tool name itself is history's job.
    assert!(complete("git", 8).is_empty());
}

#[test]
fn exact_subcommand_is_not_suggested() {
    // "git status" fully typed: nothing to complete.
    assert!(complete("git status", 8).is_empty());
}

#[test]
fn mid_argument_yields_nothing() {
    // tool + subcommand + non-flag argument: grammar never guesses arguments.
    assert!(complete("git checkout ma", 8).is_empty());
}
