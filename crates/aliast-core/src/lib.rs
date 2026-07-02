pub mod ai;
pub mod history;
pub mod predict;

use history::{HistoryStore, SuggestionContext};

/// Returns a suffix-only completion suggestion for the given command buffer
/// by looking up the best frecency-ranked matching command in the history store.
///
/// Falls back to simple prefix matching if frecency ranking returns no result.
/// Returns None if the buffer is empty, no match exists, or the match is
/// an exact duplicate of the current buffer (nothing to suggest).
pub fn suggest(store: &HistoryStore, buffer: &str, context: &SuggestionContext) -> Option<String> {
    suggest_at(store, buffer, context, 0)
}

/// Like [`suggest`], but returns the candidate at rank `skip` (0 = best),
/// powering fish-style cycling through alternatives.
pub fn suggest_at(
    store: &HistoryStore,
    buffer: &str,
    context: &SuggestionContext,
    skip: u32,
) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    // Try frecency-ranked suggestion first.
    match store.suggest_ranked_at(buffer, context, skip) {
        Ok(Some(full_command)) => {
            // An empty suffix means the ranked command is exactly what the
            // user typed -- there is nothing to complete. Do NOT fall back to the
            // raw-recency prefix match, which could surface a rarely-used longer
            // command purely because it was used more recently.
            full_command
                .strip_prefix(buffer)
                .filter(|suffix| !suffix.is_empty())
                .map(str::to_string)
        }
        // No ranked match at rank 0: fall back to a simple prefix match. Past
        // rank 0 the candidate list is simply exhausted -- no fallback.
        _ if skip == 0 => match store.suggest_prefix(buffer) {
            Ok(Some(full_command)) => full_command
                .strip_prefix(buffer)
                .filter(|suffix| !suffix.is_empty())
                .map(str::to_string),
            _ => None,
        },
        _ => None,
    }
}
