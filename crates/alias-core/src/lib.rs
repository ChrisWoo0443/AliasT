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
        let suffix = &full_command[buffer.len()..];
        if !suffix.is_empty() {
            return Some(suffix.to_string());
        }
    }

    // Fall back to simple prefix match
    match store.suggest_prefix(buffer) {
        Ok(Some(full_command)) => {
            let suffix = &full_command[buffer.len()..];
            if suffix.is_empty() {
                None
            } else {
                Some(suffix.to_string())
            }
        }
        _ => None,
    }
}
