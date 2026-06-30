pub mod claude;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;

/// Errors that can occur during AI backend operations.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// The AI backend is not reachable or not running.
    #[error("AI backend not available: {0}")]
    Unavailable(String),

    /// The AI generation request failed.
    #[error("AI generation failed: {0}")]
    GenerationFailed(String),

    /// No model has been configured for the backend.
    #[error("No model configured")]
    NoModel,
}

/// Cleans a raw model response into a bare shell command.
///
/// Models often ignore the "no markdown, no backticks" instruction and wrap
/// the command in a fenced code block, inline backticks, or a leading shell
/// prompt. Pasting that verbatim into the buffer yields a broken command, so
/// every backend runs its output through this before returning it.
///
/// Handles, in order: fenced code blocks (```` ```bash\n...\n``` ````), inline
/// backticks (`` `cmd` ``), and a leading `$ ` prompt. The result is trimmed.
pub fn sanitize_command(raw: &str) -> String {
    let mut text = raw.trim();

    // Fenced code block: drop the opening ``` (and optional language tag on the
    // same line), then the closing ```.
    if let Some(after_fence) = text.strip_prefix("```") {
        let body = match after_fence.find('\n') {
            Some(newline) => &after_fence[newline + 1..],
            None => after_fence,
        };
        text = body.trim_end().strip_suffix("```").unwrap_or(body).trim();
    }

    // Inline backticks: only unwrap when the content itself has no backtick, so
    // we don't mangle a command that legitimately uses command substitution.
    if text.len() >= 2 && text.starts_with('`') && text.ends_with('`') {
        let inner = &text[1..text.len() - 1];
        if !inner.contains('`') {
            text = inner.trim();
        }
    }

    // Leading shell prompt: "$ ls" -> "ls".
    if let Some(after_prompt) = text.strip_prefix("$ ") {
        text = after_prompt.trim();
    }

    text.to_string()
}

/// Trait for pluggable AI backends that generate shell commands from
/// natural language prompts.
#[async_trait]
pub trait AiBackend: Send + Sync {
    /// Generate a shell command from a natural language prompt.
    async fn generate(&self, prompt: &str) -> Result<String, AiError>;

    /// Check whether the backend is reachable and operational.
    async fn health_check(&self) -> Result<(), AiError>;

    /// Return the human-readable name of this backend.
    fn name(&self) -> &str;
}
