use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use super::ollama::SYSTEM_PROMPT;
use super::{AiBackend, AiError};

/// Request body for the OpenAI Chat Completions API.
#[derive(serde::Serialize)]
struct OpenAiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAiMessage>,
}

/// A single message in the OpenAI chat format.
#[derive(serde::Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// Response body from the OpenAI Chat Completions API.
#[derive(serde::Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

/// A single choice in an OpenAI response.
#[derive(serde::Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

/// The message portion of an OpenAI response choice.
#[derive(serde::Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

/// AI backend that connects to the OpenAI Chat Completions API for command generation.
pub struct OpenAiBackend {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiBackend {
    /// Create a new OpenAiBackend targeting the OpenAI API with the given key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_base_url(api_key, model, "https://api.openai.com".to_string())
    }

    /// Create a new OpenAiBackend with a custom base URL (useful for testing).
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
impl AiBackend for OpenAiBackend {
    async fn generate(&self, prompt: &str) -> Result<String, AiError> {
        let request_body = OpenAiRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|err| AiError::Unavailable(err.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::GenerationFailed(format!(
                "OpenAI returned {}: {}",
                status, body
            )));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|err| AiError::GenerationFailed(err.to_string()))?;

        let text = openai_response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .unwrap_or_default();

        Ok(text)
    }

    async fn health_check(&self) -> Result<(), AiError> {
        let health_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build health check client");

        let request_body = OpenAiRequest {
            model: self.model.clone(),
            max_tokens: 1,
            messages: vec![OpenAiMessage {
                role: "system".to_string(),
                content: "hi".to_string(),
            }],
        };

        let response = health_client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AiError::Unavailable("OpenAI API not reachable".to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AiError::Unavailable(format!(
                "OpenAI API returned {}",
                response.status()
            )))
        }
    }

    fn name(&self) -> &str {
        "openai"
    }
}
