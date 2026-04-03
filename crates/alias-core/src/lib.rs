pub mod history;

use history::HistoryStore;

/// Returns a suffix-only completion suggestion for the given command buffer
/// by looking up the most recent matching command in the history store.
///
/// Returns None if the buffer is empty, no match exists, or the match is
/// an exact duplicate of the current buffer (nothing to suggest).
pub fn suggest(store: &HistoryStore, buffer: &str) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

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
