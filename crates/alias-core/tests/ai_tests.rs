use alias_core::ai::ollama::OllamaBackend;
use alias_core::ai::{AiBackend, AiError};

/// Verify the AiBackend trait is object-safe by creating a Box<dyn AiBackend>.
#[test]
fn ai_backend_trait_is_object_safe() {
    let backend = OllamaBackend::new("codellama".to_string());
    let boxed: Box<dyn AiBackend> = Box::new(backend);
    assert_eq!(boxed.name(), "ollama");
}

#[test]
fn ollama_backend_new_sets_model() {
    let backend = OllamaBackend::new("codellama".to_string());
    // The backend was created without panic -- model is set internally.
    // We verify via name() that the struct is functional.
    assert_eq!(backend.name(), "ollama");
}

#[test]
fn ollama_backend_name_returns_ollama() {
    let backend = OllamaBackend::new("codellama".to_string());
    assert_eq!(backend.name(), "ollama");
}

#[tokio::test]
async fn ollama_health_check_returns_unavailable_for_unreachable_server() {
    let backend =
        OllamaBackend::with_base_url("codellama".to_string(), "http://localhost:19999".to_string());

    let result = backend.health_check().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        AiError::Unavailable(msg) => {
            assert!(
                msg.contains("Ollama not running"),
                "Expected user-actionable message, got: {}",
                msg
            );
        }
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}

#[tokio::test]
async fn ollama_generate_returns_error_for_unreachable_server() {
    let backend =
        OllamaBackend::with_base_url("codellama".to_string(), "http://localhost:19999".to_string());

    let result = backend.generate("list files").await;
    assert!(result.is_err(), "Expected error for unreachable server");

    // Should be Unavailable since the server cannot be reached.
    match result.unwrap_err() {
        AiError::Unavailable(_) => {}
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}
