mod parser;
mod store;

pub use parser::{HistoryEntry, parse_history_bytes, parse_history_file};
pub use store::HistoryStore;

/// Environmental context for smarter suggestion ranking.
#[derive(Debug, Clone, Default)]
pub struct SuggestionContext {
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub git_branch: Option<String>,
}
