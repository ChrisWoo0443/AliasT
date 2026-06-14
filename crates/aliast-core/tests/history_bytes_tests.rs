use aliast_core::history::parse_history_bytes;

#[test]
fn parses_plain_ascii_bytes() {
    let entries = parse_history_bytes(b"git status\nls -la\n");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].command, "git status");
    assert_eq!(entries[1].command, "ls -la");
}

#[test]
fn demetafies_zsh_metafied_utf8() {
    // '☕' is UTF-8 E2 98 95. zsh metafies the 0x98 and 0x95 bytes by writing a
    // Meta byte (0x83) followed by the original byte XOR 0x20 (B8, B5). A plain
    // read_to_string chokes on the 0x83; de-metafication restores the char.
    let line = b"echo \xe2\x83\xb8\x83\xb5\n";
    let entries = parse_history_bytes(line);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].command, "echo ☕");
}

#[test]
fn invalid_utf8_does_not_abort_whole_import() {
    // A stray non-UTF-8 byte on one line must not discard the other entries
    // (the previous behavior read the whole file as UTF-8 and bailed entirely).
    let content = b"good one\n\xff\xfe\ngood two\n";
    let entries = parse_history_bytes(content);
    let commands: Vec<&str> = entries.iter().map(|e| e.command.as_str()).collect();
    assert!(commands.contains(&"good one"), "got {commands:?}");
    assert!(commands.contains(&"good two"), "got {commands:?}");
}
