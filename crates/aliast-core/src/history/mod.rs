mod parser;
mod store;

pub use parser::{parse_history_file, HistoryEntry};
pub use store::HistoryStore;

/// Environmental context for smarter suggestion ranking.
#[derive(Debug, Clone, Default)]
pub struct SuggestionContext {
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub git_branch: Option<String>,
}
