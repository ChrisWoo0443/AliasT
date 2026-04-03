use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use super::ollama::SYSTEM_PROMPT;
use super::{AiBackend, AiError};

/// Request body for the Anthropic Messages API.
#[derive(serde::Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

/// A single message in the Claude chat format.
#[derive(serde::Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// Response body from the Anthropic Messages API.
#[derive(serde::Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

/// A content block in a Claude response.
#[derive(serde::Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    _content_type: String,
    text: String,
}

/// AI backend that connects to the Anthropic Claude API for command generation.
pub struct ClaudeBackend {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl ClaudeBackend {
    /// Create a new ClaudeBackend targeting the Anthropic API with the given key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_base_url(api_key, model, "https://api.anthropic.com".to_string())
    }

    /// Create a new ClaudeBackend with a custom base URL (useful for testing).
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            api_key,
            base_url,
            model,
        }
    }
}

#[async_trait]
impl AiBackend for ClaudeBackend {
    async fn generate(&self, prompt: &str) -> Result<String, AiError> {
        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            system: SYSTEM_PROMPT.to_string(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|err| AiError::Unavailable(err.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::GenerationFailed(format!(
                "Claude returned {}: {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|err| AiError::GenerationFailed(err.to_string()))?;

        let text = claude_response
            .content
            .first()
            .map(|block| block.text.trim().to_string())
            .unwrap_or_default();

        Ok(text)
    }

    async fn health_check(&self) -> Result<(), AiError> {
        let health_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build health check client");

        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1,
            system: String::new(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
        };

        let response = health_client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AiError::Unavailable("Claude API not reachable".to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AiError::Unavailable(format!(
                "Claude API returned {}",
                response.status()
            )))
        }
    }

    fn name(&self) -> &str {
        "claude"
    }
}
