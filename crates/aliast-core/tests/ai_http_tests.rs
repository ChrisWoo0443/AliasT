//! HTTP-level tests for the AI backends against a mock server, covering the
//! success path (with fence stripping), non-2xx handling, and empty responses.

use aliast_core::ai::ollama::OllamaBackend;
use aliast_core::ai::openai::OpenAiBackend;
use aliast_core::ai::{AiBackend, AiError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn ollama_generate_strips_fences_from_success_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"message":{"content":"```bash\nls -la\n```"}}"#),
        )
        .mount(&server)
        .await;

    let backend = OllamaBackend::with_base_url("llama3.2".to_string(), server.uri());
    let command = backend.generate("list files").await.unwrap();

    assert_eq!(command, "ls -la");
}

#[tokio::test]
async fn ollama_generate_maps_non_2xx_to_generation_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("model overloaded"))
        .mount(&server)
        .await;

    let backend = OllamaBackend::with_base_url("llama3.2".to_string(), server.uri());
    let error = backend.generate("list files").await.unwrap_err();

    match error {
        AiError::GenerationFailed(message) => assert!(message.contains("500")),
        other => panic!("expected GenerationFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn ollama_generate_errors_on_empty_content() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"message":{"content":""}}"#))
        .mount(&server)
        .await;

    let backend = OllamaBackend::with_base_url("llama3.2".to_string(), server.uri());
    assert!(matches!(
        backend.generate("do nothing").await.unwrap_err(),
        AiError::GenerationFailed(_)
    ));
}

#[tokio::test]
async fn openai_generate_errors_on_empty_choices() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"choices":[]}"#))
        .mount(&server)
        .await;

    let backend =
        OpenAiBackend::with_base_url("sk-test".to_string(), "gpt-4o".to_string(), server.uri());
    assert!(matches!(
        backend.generate("do nothing").await.unwrap_err(),
        AiError::GenerationFailed(_)
    ));
}
