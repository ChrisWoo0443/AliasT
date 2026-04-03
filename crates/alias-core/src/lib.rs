pub mod history;

/// Returns a hardcoded completion suggestion for the given command buffer.
///
/// This is a Phase 1 placeholder that provides static suggestions for
/// end-to-end testing of the ghost text pipeline. It will be replaced
/// with history-based and AI-powered suggestions in later phases.
pub fn suggest(buffer: &str) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    if buffer.starts_with("git ch") {
        Some("eckout main".to_string())
    } else if buffer.starts_with("git co") {
        Some("mmit -m \"\"".to_string())
    } else if buffer == "ls" {
        Some(" -la".to_string())
    } else if buffer.starts_with("cd ") {
        Some("..".to_string())
    } else {
        None
    }
}
