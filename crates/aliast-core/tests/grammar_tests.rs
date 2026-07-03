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

#[test]
fn completes_flag_for_known_subcommand() {
    assert_eq!(complete("git commit --am", 8), vec!["git commit --amend"]);
}

#[test]
fn flag_candidates_include_globals_after_subcommand_flags() {
    // cargo build: subcommand flags first (table order), then global flags.
    let got = complete("cargo build --", 16);
    assert_eq!(
        got,
        vec![
            "cargo build --release",
            "cargo build --workspace",
            "cargo build --all-targets",
            "cargo build --help",
            "cargo build --version",
            "cargo build --locked",
            "cargo build --offline",
        ]
    );
}

#[test]
fn flag_directly_after_tool_completes_global_flags() {
    assert_eq!(complete("git --ver", 8), vec!["git --version"]);
}

#[test]
fn sudo_prefix_is_stripped_for_matching() {
    assert_eq!(complete("sudo git sw", 8), vec!["sudo git switch"]);
}

#[test]
fn quotes_and_separators_yield_nothing() {
    assert!(complete("git commit -m \"fix", 8).is_empty());
    assert!(complete("cd x && git sw", 8).is_empty());
    assert!(complete("git s | grep x", 8).is_empty());
    assert!(complete("echo a; git s", 8).is_empty());
    assert!(complete("git commit -m 'w", 8).is_empty());
    assert!(complete("git s`whoami`", 8).is_empty());
    assert!(complete("git commit -m fix\\", 8).is_empty());
}

#[test]
fn unknown_subcommand_flag_yields_nothing() {
    assert!(complete("git frobnicate --am", 8).is_empty());
}

#[test]
fn grammar_pack_covers_the_common_tools() {
    // One spot check per tool added in this task.
    assert_eq!(complete("docker ru", 8), vec!["docker run"]);
    assert_eq!(complete("brew inst", 8), vec!["brew install"]);
    assert_eq!(complete("npm inst", 8), vec!["npm install"]);
    assert_eq!(complete("pnpm ad", 8), vec!["pnpm add"]);
    assert_eq!(complete("yarn ad", 8), vec!["yarn add"]);
    assert_eq!(complete("kubectl app", 8), vec!["kubectl apply"]);
    assert_eq!(complete("gh p", 8), vec!["gh pr"]);
    assert_eq!(complete("go bu", 8), vec!["go build"]);
    assert_eq!(complete("rustup upd", 8), vec!["rustup update"]);
    assert_eq!(complete("pip inst", 8), vec!["pip install"]);
    assert_eq!(complete("uv sy", 8), vec!["uv sync"]);
    assert_eq!(complete("terraform pl", 8), vec!["terraform plan"]);
    assert_eq!(complete("tmux new-s", 8), vec!["tmux new-session"]);
}
