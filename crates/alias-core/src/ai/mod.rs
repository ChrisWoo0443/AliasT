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
