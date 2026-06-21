//! `aliast doctor` must not report a healthy Ollama setup when the configured
//! model was never pulled.

use aliast_daemon::doctor::check_ollama_at;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn ollama_check_passes_when_model_is_available() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"models":[{"name":"llama3.2:latest"}]}"#),
        )
        .mount(&server)
        .await;

    let check = check_ollama_at(&server.uri(), "llama3.2").await;

    assert!(check.passed, "{}", check.detail);
}

#[tokio::test]
async fn ollama_check_fails_when_model_not_pulled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"models":[{"name":"mistral:latest"}]}"#),
        )
        .mount(&server)
        .await;

    let check = check_ollama_at(&server.uri(), "llama3.2").await;

    assert!(!check.passed, "should fail when model is absent");
    assert!(check.fix.unwrap().contains("ollama pull llama3.2"));
}

#[tokio::test]
async fn ollama_check_fails_when_unreachable() {
    // Nothing is listening here.
    let check = check_ollama_at("http://127.0.0.1:1", "llama3.2").await;
    assert!(!check.passed);
}
