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

/// Cleans a model's raw response into a single shell command.
///
/// Models routinely ignore the "no markdown" instruction (local models most of
/// all), so strip a surrounding triple-backtick fence (with optional language
/// tag) and stray wrapping single backticks, then trim. An empty result becomes
/// a `GenerationFailed` error so the caller shows a clear message instead of
/// dropping the user into an empty review buffer. Prose is intentionally left
/// untouched -- guessing which line is "the command" risks running the wrong one.
pub fn sanitize_command(raw: &str) -> Result<String, AiError> {
    let unfenced = strip_code_fence(raw.trim());
    let cleaned = strip_wrapping_backticks(unfenced.trim()).trim();

    if cleaned.is_empty() {
        return Err(AiError::GenerationFailed(
            "model returned an empty command".to_string(),
        ));
    }
    Ok(cleaned.to_string())
}

/// Strip a surrounding ```-fenced block (with an optional language tag line).
fn strip_code_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    let rest = rest.trim_end();
    let rest = rest.strip_suffix("```").unwrap_or(rest);
    // Drop the first line (language tag or blank) if the fence spanned lines.
    match rest.split_once('\n') {
        Some((_first_line, body)) => body,
        None => rest,
    }
}

/// Strip a single pair of wrapping backticks: `cmd` -> cmd.
fn strip_wrapping_backticks(text: &str) -> &str {
    text.strip_prefix('`')
        .and_then(|inner| inner.strip_suffix('`'))
        .unwrap_or(text)
}

/// Trait for pluggable AI backends that generate shell commands from
/// natural language prompts.
#[async_trait]
pub trait AiBackend: Send + Sync {
    /// Generate a shell command from a natural language prompt.
    async fn generate(&self, prompt: &str) -> Result<String, AiError>;

    /// Generate a shell command, emitting raw text fragments on `chunk_tx` as
    /// they arrive, and returning the final sanitized command. Backends without
    /// streaming support fall back to [`generate`](Self::generate) (no chunks).
    async fn generate_stream(
        &self,
        prompt: &str,
        chunk_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, AiError> {
        let _ = chunk_tx; // default: non-streaming fallback
        self.generate(prompt).await
    }

    /// Check whether the backend is reachable and operational.
    async fn health_check(&self) -> Result<(), AiError>;

    /// Return the human-readable name of this backend.
    fn name(&self) -> &str;
}
