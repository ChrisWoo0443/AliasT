//! Directory completion for directory-centric commands (cd, ls, pushd,
//! mkdir, rmdir): complete the trailing token as a real subdirectory,
//! directories only, trailing '/' appended so the user can keep typing.
//!
//! Fallibility contract: this runs on the keystroke path -- every error
//! (unreadable dir, no cwd, weird input) degrades to "no candidates".

use std::path::PathBuf;

/// Commands whose trailing argument is completed as a directory.
const ALLOWLIST: &[&str] = &["cd", "ls", "pushd", "mkdir", "rmdir"];

/// Latency guard: stop scanning pathological directories after this many
/// entries rather than blowing the ghost-text budget.
const SCAN_CAP: usize = 2048;

/// Suggested text is executed as-is when accepted: a name containing
/// whitespace would break the command into extra words, and a name planted
/// with one of these shell-significant characters (trivially done via
/// `git clone`/`unzip`/etc.) would turn innocent ghost text into a compound
/// command. Filtering candidates out -- not escaping them -- matches this
/// module's conservative-guard philosophy; escaping is a possible v2.
const UNSAFE_NAME_CHARS: &[char] = &[
    '"', '\'', '`', '\\', ';', '|', '&', '$', '(', ')', '<', '>', '*', '?', '#', '!',
];

/// Cheap pre-check so callers can skip the cd-history SQL query entirely
/// for buffers that can never produce a directory completion.
pub fn is_eligible(buffer: &str) -> bool {
    if buffer.contains(['"', '\'', '`', '\\', ';', '|', '&']) {
        return false;
    }
    let trailing_space = buffer.ends_with(char::is_whitespace);
    let mut tokens = buffer.split_whitespace();
    let Some(first) = tokens.next() else {
        return false;
    };
    if !ALLOWLIST.contains(&first) {
        return false;
    }
    // A space after the command is required; completing the command word
    // itself is history's job. And a partial token that starts with '-' is
    // a flag, not a path -- reject here so callers skip the cd-history SQL
    // query entirely instead of running it only for `complete` to bail on
    // the same check.
    match tokens.last() {
        Some(last) if !trailing_space => !last.starts_with('-'),
        Some(_) => true,
        None => trailing_space,
    }
}

/// Complete the trailing token of `buffer` as a directory. Returns at most
/// `limit` full command strings extending `buffer`, each ending in '/'.
/// Candidates matching a cd-history target sort first, in frequency order;
/// ties stay alphabetical. Pass `&[]` for pure alphabetical filesystem
/// completion.
pub fn complete(
    buffer: &str,
    cwd: Option<&str>,
    cd_history: &[String],
    limit: usize,
) -> Vec<String> {
    if !is_eligible(buffer) {
        return Vec::new();
    }

    let trailing_space = buffer.ends_with(char::is_whitespace);
    let partial = if trailing_space {
        ""
    } else {
        buffer.split_whitespace().last().unwrap_or("")
    };
    // (Flag partials are rejected by is_eligible above, before the
    // cd-history SQL query even runs; no need to re-check here.)

    // Split the partial into the parent to scan and the name prefix to match:
    // "crates/al" -> scan "crates", match "al"; "al" -> scan ".", match "al".
    let (typed_parent, prefix) = match partial.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", partial),
    };

    // Resolve the directory to scan. '~/...' expands via HOME; absolute paths
    // stand alone; relative paths need a cwd.
    let scan_dir: PathBuf = if let Some(rest) = typed_parent.strip_prefix("~/") {
        let Ok(home) = std::env::var("HOME") else {
            return Vec::new();
        };
        PathBuf::from(home).join(rest)
    } else if typed_parent == "~" {
        let Ok(home) = std::env::var("HOME") else {
            return Vec::new();
        };
        PathBuf::from(home)
    } else if partial.starts_with('/') {
        // rsplit_once ate the leading '/' when parent is "", e.g. "/us" ->
        // ("", "us"): scan the root.
        if typed_parent.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(typed_parent)
        }
    } else {
        let Some(cwd) = cwd else {
            return Vec::new();
        };
        if typed_parent.is_empty() {
            PathBuf::from(cwd)
        } else {
            PathBuf::from(cwd).join(typed_parent)
        }
    };

    let Ok(entries) = std::fs::read_dir(&scan_dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = Vec::new();
    for entry in entries.take(SCAN_CAP).flatten() {
        // Directories only; follows symlinks so a linked dir still completes.
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue; // non-UTF-8 names cannot round-trip through the protocol
        };
        if !name.starts_with(prefix) {
            continue;
        }
        if !prefix.starts_with('.') && name.starts_with('.') {
            continue;
        }
        if name.chars().any(|c| c.is_whitespace() || c.is_control())
            || name.contains(UNSAFE_NAME_CHARS)
        {
            continue;
        }
        names.push(name);
    }
    names.sort();

    // Blend in navigation history: a candidate whose typed relative path
    // matches a `cd` target the user actually ran from this cwd sorts first,
    // in cd_history order (which is frequency order, per the store query).
    let history_targets: Vec<&str> = cd_history
        .iter()
        .filter_map(|cmd| extract_cd_target(cmd))
        .collect();
    let history_rank = |name: &str| -> usize {
        let typed = if typed_parent.is_empty() {
            name.to_string()
        } else {
            format!("{typed_parent}/{name}")
        };
        history_targets
            .iter()
            .position(|target| *target == typed)
            .unwrap_or(usize::MAX)
    };
    names.sort_by_cached_key(|name| history_rank(name)); // stable: ties stay alphabetical

    names
        .into_iter()
        .take(limit)
        .map(|name| format!("{}{}/", buffer, &name[prefix.len()..]))
        .collect()
}

/// Extract the target of a simple `cd <target>` command; quoted, escaped, or
/// option-bearing forms return None (they cannot match a plain dir name).
fn extract_cd_target(cmd: &str) -> Option<&str> {
    let target = cmd.strip_prefix("cd ")?.trim();
    if target.is_empty() || target.contains(['"', '\'', '`', '\\']) || target.starts_with('-') {
        return None;
    }
    Some(target.trim_end_matches('/'))
}
