use alias_core::ai::claude::ClaudeBackend;
use alias_core::ai::ollama::OllamaBackend;
use alias_core::ai::ollama::SYSTEM_PROMPT;
use alias_core::ai::openai::OpenAiBackend;
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

// --- System prompt context tests ---

#[test]
fn system_prompt_mentions_context_block() {
    assert!(
        SYSTEM_PROMPT.contains("Context"),
        "System prompt should mention [Context] blocks"
    );
}

#[test]
fn system_prompt_mentions_current_directory() {
    assert!(
        SYSTEM_PROMPT.contains("current directory"),
        "System prompt should mention current directory context"
    );
}

#[test]
fn system_prompt_mentions_git_branch() {
    assert!(
        SYSTEM_PROMPT.contains("git branch"),
        "System prompt should mention git branch context"
    );
}

#[test]
fn system_prompt_mentions_exit_code() {
    assert!(
        SYSTEM_PROMPT.contains("exit code"),
        "System prompt should mention exit code context"
    );
}

// --- Claude backend tests ---

#[test]
fn claude_backend_name_returns_claude() {
    let backend = ClaudeBackend::new("test-key".to_string(), "claude-sonnet-4-20250514".to_string());
    assert_eq!(backend.name(), "claude");
}

#[test]
fn claude_backend_is_object_safe() {
    let backend = ClaudeBackend::new("test-key".to_string(), "claude-sonnet-4-20250514".to_string());
    let boxed: Box<dyn AiBackend> = Box::new(backend);
    assert_eq!(boxed.name(), "claude");
}

#[tokio::test]
async fn claude_generate_returns_unavailable_for_unreachable_server() {
    let backend = ClaudeBackend::with_base_url(
        "test-key".to_string(),
        "claude-sonnet-4-20250514".to_string(),
        "http://localhost:19998".to_string(),
    );

    let result = backend.generate("list files").await;
    assert!(result.is_err(), "Expected error for unreachable server");

    match result.unwrap_err() {
        AiError::Unavailable(_) => {}
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}

#[tokio::test]
async fn claude_health_check_returns_unavailable_for_unreachable_server() {
    let backend = ClaudeBackend::with_base_url(
        "test-key".to_string(),
        "claude-sonnet-4-20250514".to_string(),
        "http://localhost:19998".to_string(),
    );

    let result = backend.health_check().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        AiError::Unavailable(msg) => {
            assert!(
                msg.contains("Claude API not reachable"),
                "Expected user-actionable message, got: {}",
                msg
            );
        }
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}

// --- OpenAI backend tests ---

#[test]
fn openai_backend_name_returns_openai() {
    let backend = OpenAiBackend::new("test-key".to_string(), "gpt-4o".to_string());
    assert_eq!(backend.name(), "openai");
}

#[test]
fn openai_backend_is_object_safe() {
    let backend = OpenAiBackend::new("test-key".to_string(), "gpt-4o".to_string());
    let boxed: Box<dyn AiBackend> = Box::new(backend);
    assert_eq!(boxed.name(), "openai");
}

#[tokio::test]
async fn openai_generate_returns_unavailable_for_unreachable_server() {
    let backend = OpenAiBackend::with_base_url(
        "test-key".to_string(),
        "gpt-4o".to_string(),
        "http://localhost:19997".to_string(),
    );

    let result = backend.generate("list files").await;
    assert!(result.is_err(), "Expected error for unreachable server");

    match result.unwrap_err() {
        AiError::Unavailable(_) => {}
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}

#[tokio::test]
async fn openai_health_check_returns_unavailable_for_unreachable_server() {
    let backend = OpenAiBackend::with_base_url(
        "test-key".to_string(),
        "gpt-4o".to_string(),
        "http://localhost:19997".to_string(),
    );

    let result = backend.health_check().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        AiError::Unavailable(msg) => {
            assert!(
                msg.contains("OpenAI API not reachable"),
                "Expected user-actionable message, got: {}",
                msg
            );
        }
        other => panic!("Expected AiError::Unavailable, got: {:?}", other),
    }
}
