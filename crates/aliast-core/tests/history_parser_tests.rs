use aliast_core::history::{parse_history_file, HistoryEntry};

#[test]
fn parse_plain_format() {
    let content = "git status\nls -la\n";
    let entries = parse_history_file(content);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].command, "git status");
    assert_eq!(entries[0].timestamp, None);
    assert_eq!(entries[1].command, "ls -la");
    assert_eq!(entries[1].timestamp, None);
}

#[test]
fn parse_extended_history_format() {
    let content = ": 1700000000:0;git status\n";
    let entries = parse_history_file(content);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].command, "git status");
    assert_eq!(entries[0].timestamp, Some(1700000000));
}

#[test]
fn parse_multiline_command() {
    let content = "for i in a b c; do\\\necho $i\\\ndone\n";
    let entries = parse_history_file(content);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].command, "for i in a b c; do\necho $i\ndone");
}

#[test]
fn parse_skips_empty_lines() {
    let content = "git status\n\n  \n\nls -la\n";
    let entries = parse_history_file(content);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].command, "git status");
    assert_eq!(entries[1].command, "ls -la");
}

#[test]
fn parse_mixed_format() {
    let content = ": 1700000000:0;git status\nls -la\n: 1700000001:0;cargo build\n";
    let entries = parse_history_file(content);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].command, "git status");
    assert_eq!(entries[0].timestamp, Some(1700000000));
    assert_eq!(entries[1].command, "ls -la");
    assert_eq!(entries[1].timestamp, None);
    assert_eq!(entries[2].command, "cargo build");
    assert_eq!(entries[2].timestamp, Some(1700000001));
}
