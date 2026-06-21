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

    // Try frecency-ranked suggestion first.
    match store.suggest_ranked(buffer, context) {
        Ok(Some(full_command)) => {
            // An empty suffix means the top-ranked command is exactly what the
            // user typed -- there is nothing to complete. Do NOT fall back to the
            // raw-recency prefix match, which could surface a rarely-used longer
            // command purely because it was used more recently.
            full_command
                .strip_prefix(buffer)
                .filter(|suffix| !suffix.is_empty())
                .map(str::to_string)
        }
        // No ranked match at all: fall back to a simple prefix match.
        _ => match store.suggest_prefix(buffer) {
            Ok(Some(full_command)) => full_command
                .strip_prefix(buffer)
                .filter(|suffix| !suffix.is_empty())
                .map(str::to_string),
            _ => None,
        },
    }
}
