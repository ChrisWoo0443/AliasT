/// A single entry from a zsh history file.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// The command that was executed.
    pub command: String,
    /// Unix timestamp of execution, if available (from EXTENDED_HISTORY format).
    pub timestamp: Option<i64>,
}

/// Parses zsh history file content into a list of history entries.
///
/// Handles both plain format and EXTENDED_HISTORY format (`: timestamp:duration;command`),
/// including multiline commands with backslash continuation.
pub fn parse_history_file(content: &str) -> Vec<HistoryEntry> {
    todo!()
}

/// Attempts to parse a line as EXTENDED_HISTORY format.
/// Returns None if the line does not match the expected pattern.
fn parse_extended_line(line: &str) -> Option<HistoryEntry> {
    todo!()
}
