//! Bundled grammar pack: curated subcommand/flag tables for common CLI tools.
//! Ordering within each list is the rank order (most common first).

use super::grammar::{Subcommand, ToolGrammar};

macro_rules! sub {
    ($name:literal) => {
        Subcommand {
            name: $name,
            flags: &[],
        }
    };
    ($name:literal, $flags:expr) => {
        Subcommand {
            name: $name,
            flags: $flags,
        }
    };
}

pub(crate) static GRAMMARS: &[ToolGrammar] = &[
    ToolGrammar {
        name: "git",
        global_flags: &["--help", "--version"],
        subcommands: &[
            sub!("status", &["-s", "--short", "-b"]),
            sub!("add", &["-A", "-p", "-u"]),
            sub!("commit", &["-m", "-a", "--amend", "--no-edit", "-v"]),
            sub!("push", &["-u", "--force-with-lease", "--tags", "--dry-run"]),
            sub!("pull", &["--rebase", "--ff-only"]),
            sub!("checkout", &["-b", "--"]),
            sub!("branch", &["-d", "-D", "-a", "--show-current"]),
            sub!("switch", &["-c", "--detach"]),
            sub!("diff", &["--staged", "--stat", "--name-only"]),
            sub!("log", &["--oneline", "--graph", "-p", "--stat"]),
            sub!("stash", &["--include-untracked"]),
            sub!("fetch", &["--all", "--prune"]),
            sub!("merge", &["--no-ff", "--abort", "--squash"]),
            sub!("rebase", &["--continue", "--abort", "--onto"]),
            sub!("clone", &["--depth", "--branch"]),
            sub!("restore", &["--staged", "--source"]),
            sub!("reset", &["--hard", "--soft", "--mixed"]),
            sub!("remote", &["-v"]),
            sub!("tag", &["-a", "-d", "-l"]),
            sub!("show", &["--stat", "--name-only"]),
            sub!("cherry-pick", &["--continue", "--abort"]),
            sub!("revert", &["--no-edit", "--continue"]),
            sub!("rm", &["-r", "--cached"]),
            sub!("mv", &[]),
            sub!("init", &["--bare"]),
            sub!("blame", &["-L"]),
            sub!("grep", &["-n", "-i"]),
            sub!("reflog", &[]),
            sub!("worktree", &[]),
            sub!("bisect", &[]),
        ],
    },
    ToolGrammar {
        name: "cargo",
        global_flags: &["--help", "--version", "--locked", "--offline"],
        subcommands: &[
            sub!(
                "build",
                &["--release", "--workspace", "-p", "--all-targets"]
            ),
            sub!("run", &["--release", "-p", "--bin", "--example"]),
            sub!("test", &["--workspace", "--release", "-p", "--no-run"]),
            sub!("check", &["--workspace", "--all-targets"]),
            sub!("clippy", &["--workspace", "--all-targets", "--fix"]),
            sub!("fmt", &["--all", "--check"]),
            sub!("add", &["--dev", "--features", "--no-default-features"]),
            sub!("remove", &["--dev"]),
            sub!("update", &["-p", "--dry-run"]),
            sub!("install", &["--path", "--force"]),
            sub!("new", &["--lib", "--bin"]),
            sub!("doc", &["--open", "--no-deps"]),
            sub!("clean", &["--release"]),
            sub!("bench", &[]),
            sub!("publish", &["--dry-run"]),
        ],
    },
];
