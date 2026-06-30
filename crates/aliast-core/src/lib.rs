pub mod ai;
pub mod history;

use history::{HistoryStore, SuggestionContext};

/// Returns a suffix-only completion suggestion for the given command buffer
/// by looking up the best frecency-ranked matching command in the history store.
///
/// Falls back to simple prefix matching if frecency ranking returns no result.
/// Returns None if the buffer is empty, no match exists, or the match is
/// an exact duplicate of the current buffer (nothing to suggest).
pub fn suggest(store: &HistoryStore, buffer: &str, context: &SuggestionContext) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    // Try frecency-ranked suggestion first
    if let Ok(Some(full_command)) = store.suggest_ranked(buffer, context) {
        let ranked_suffix = nonempty_suffix(&full_command, buffer);
        if ranked_suffix.is_some() {
            return ranked_suffix;
        }
    }

    // Fall back to simple prefix match
    match store.suggest_prefix(buffer) {
        Ok(Some(full_command)) => nonempty_suffix(&full_command, buffer),
        _ => None,
    }
}

/// Returns the portion of `command` that extends past `buffer`, or None if the
/// command does not start with `buffer` or adds nothing.
///
/// Uses `strip_prefix` rather than byte slicing so a match that unexpectedly
/// isn't prefixed by the buffer yields None instead of panicking the daemon
/// (e.g. on a non-char-boundary slice).
fn nonempty_suffix(command: &str, buffer: &str) -> Option<String> {
    command
        .strip_prefix(buffer)
        .filter(|suffix| !suffix.is_empty())
        .map(str::to_string)
}
