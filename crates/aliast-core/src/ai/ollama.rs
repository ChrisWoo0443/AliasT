use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use super::{AiBackend, AiError};

/// System prompt that instructs the model to output only a shell command.
pub const SYSTEM_PROMPT: &str = "\
You are a shell command generator for macOS zsh. \
The user will describe what they want to do in plain English. \
Output ONLY the shell command. No explanation, no markdown, no backticks, no commentary. \
If the request is ambiguous, output the most likely intended command. \
If the request cannot be translated to a shell command, output: echo 'Could not generate command' \
Always prefer standard macOS/BSD tools. Use common flags. \
The user's message may start with a [Context] block containing their current directory, \
git branch, and last exit code. Use this context to generate more relevant commands.";

/// Request body for the Ollama /api/chat endpoint.
#[derive(serde::Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

/// A single message in the Ollama chat format.
#[derive(serde::Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Response body from the Ollama /api/chat endpoint.
#[derive(serde::Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

/// The message portion of an Ollama chat response.
#[derive(serde::Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

/// AI backend that connects to a local Ollama instance for command generation.
pub struct OllamaBackend {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaBackend {
    /// Create a new OllamaBackend targeting localhost:11434 with the given model.
    pub fn new(model: String) -> Self {
        Self::with_base_url(model, "http://localhost:11434".to_string())
    }

    /// Create a new OllamaBackend with a custom base URL (useful for testing).
    pub fn with_base_url(model: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url,
            model,
        }
    }
}

#[async_trait]
impl AiBackend for OllamaBackend {
    async fn generate(&self, prompt: &str) -> Result<String, AiError> {
        let request_body = OllamaChatRequest {
            model: self.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request_body)
            .send()
            .await
            .map_err(|err| AiError::Unavailable(err.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::GenerationFailed(format!(
                "Ollama returned {}: {}",
                status, body
            )));
        }

        let chat_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|err| AiError::GenerationFailed(err.to_string()))?;

        Ok(chat_response.message.content.trim().to_string())
    }

    async fn health_check(&self) -> Result<(), AiError> {
        let health_client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("failed to build health check client");

        let response = health_client
            .get(format!("{}/", self.base_url))
            .send()
            .await
            .map_err(|_| {
                AiError::Unavailable(
                    "Ollama not running -- start with: ollama serve".to_string(),
                )
            })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AiError::Unavailable(
                "Ollama not running -- start with: ollama serve".to_string(),
            ))
        }
    }

    fn name(&self) -> &str {
        "ollama"
    }
}
