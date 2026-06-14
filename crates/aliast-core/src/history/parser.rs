/// A single entry from a zsh history file.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// The command that was executed.
    pub command: String,
    /// Unix timestamp of execution, if available (from EXTENDED_HISTORY format).
    pub timestamp: Option<i64>,
}

/// Parses raw zsh history file bytes into a list of history entries.
///
/// zsh stores history in a "metafied" encoding: bytes that collide with its
/// internal token range are written as a Meta byte (0x83) followed by the
/// original byte XOR 0x20 (so a UTF-8 emoji or box-drawing char produces stray
/// 0x83 bytes on disk). Reading such a file as UTF-8 fails, which previously
/// aborted the entire first-run import. This de-metafies first, then decodes
/// lossily so a single bad byte cannot discard the whole history.
pub fn parse_history_bytes(bytes: &[u8]) -> Vec<HistoryEntry> {
    let unmetafied = unmetafy(bytes);
    let decoded = String::from_utf8_lossy(&unmetafied);
    parse_history_file(&decoded)
}

/// Reverses zsh's history metafication (see `parse_history_bytes`).
fn unmetafy(bytes: &[u8]) -> Vec<u8> {
    const META: u8 = 0x83;
    let mut out = Vec::with_capacity(bytes.len());
    let mut iter = bytes.iter().copied();
    while let Some(byte) = iter.next() {
        if byte == META {
            // The next byte was stored XOR 0x20; a trailing lone Meta is dropped.
            if let Some(escaped) = iter.next() {
                out.push(escaped ^ 0x20);
            }
        } else {
            out.push(byte);
        }
    }
    out
}

/// Parses zsh history file content into a list of history entries.
///
/// Handles both plain format and EXTENDED_HISTORY format (`: timestamp:duration;command`),
/// including multiline commands with backslash continuation.
pub fn parse_history_file(content: &str) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        let mut accumulated_line = String::new();

        // Handle backslash continuation: if line ends with '\' (not '\\'), join with next
        loop {
            let current_line = lines[index];
            index += 1;

            if current_line.ends_with('\\') && !current_line.ends_with("\\\\") {
                // Strip the trailing backslash and append
                accumulated_line.push_str(&current_line[..current_line.len() - 1]);
                accumulated_line.push('\n');
                if index >= lines.len() {
                    break;
                }
            } else {
                accumulated_line.push_str(current_line);
                break;
            }
        }

        // Skip empty/whitespace-only lines
        if accumulated_line.trim().is_empty() {
            continue;
        }

        // Try EXTENDED_HISTORY format first, fall back to plain
        if let Some(entry) = parse_extended_line(&accumulated_line) {
            entries.push(entry);
        } else {
            entries.push(HistoryEntry {
                command: accumulated_line,
                timestamp: None,
            });
        }
    }

    entries
}

/// Attempts to parse a line as EXTENDED_HISTORY format.
/// Expected format: `: timestamp:duration;command`
/// Returns None if the line does not match the expected pattern.
fn parse_extended_line(line: &str) -> Option<HistoryEntry> {
    // Must start with ": "
    let rest = line.strip_prefix(": ")?;

    // Find the first ':' separating timestamp from duration
    let colon_position = rest.find(':')?;
    let timestamp_str = &rest[..colon_position];
    let timestamp: i64 = timestamp_str.parse().ok()?;

    // Find the ';' separating duration from command
    let after_colon = &rest[colon_position + 1..];
    let semicolon_position = after_colon.find(';')?;
    let command = &after_colon[semicolon_position + 1..];

    if command.is_empty() {
        return None;
    }

    Some(HistoryEntry {
        command: command.to_string(),
        timestamp: Some(timestamp),
    })
}
