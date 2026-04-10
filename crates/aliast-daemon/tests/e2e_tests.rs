//! End-to-end integration tests proving the full NDJSON request/response loop.
//!
//! These tests start a real daemon server on a temp socket with a SQLite-backed
//! HistoryStore, connect a client, send NDJSON requests, and validate the
//! responses -- proving the complete communication loop works over Unix domain
//! sockets with real history data.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use aliast_core::ai::{AiBackend, AiError};
use aliast_core::history::HistoryStore;
use aliast_daemon::DaemonState;
use aliast_daemon::server;
use aliast_protocol::Response;

/// Mock AI backend that captures the prompt and returns a fixed response.
struct MockAiBackend {
    response: String,
    captured_prompt: Arc<Mutex<Option<String>>>,
}

impl MockAiBackend {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            captured_prompt: Arc::new(Mutex::new(None)),
        }
    }

    fn captured_prompt(&self) -> Arc<Mutex<Option<String>>> {
        self.captured_prompt.clone()
    }
}

#[async_trait]
impl AiBackend for MockAiBackend {
    async fn generate(&self, prompt: &str) -> Result<String, AiError> {
        *self.captured_prompt.lock().unwrap() = Some(prompt.to_string());
        Ok(self.response.clone())
    }

    async fn health_check(&self) -> Result<(), AiError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

/// Start daemon on a temp socket with a fresh HistoryStore, returning path,
/// cancel token, and temp dir guard.
async fn spawn_daemon() -> (std::path::PathBuf, CancellationToken, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("alias.sock");
    let db_path = temp_dir.path().join("history.db");
    let cancel_token = CancellationToken::new();

    let store = HistoryStore::open(&db_path).unwrap();
    let shared_store = Arc::new(Mutex::new(store));

    let state = DaemonState {
        store: shared_store,
        ai_backend: None,
        cancel_token: cancel_token.clone(),
        enabled: Arc::new(AtomicBool::new(true)),
    };

    let server_path = socket_path.clone();
    tokio::spawn(async move {
        server::run_server(&server_path, state).await.unwrap();
    });

    // Wait for server to bind the socket
    tokio::time::sleep(Duration::from_millis(100)).await;

    (socket_path, cancel_token, temp_dir)
}

/// Start daemon on a temp socket with a mock AI backend, returning path,
/// cancel token, temp dir guard, and captured prompt reference.
async fn spawn_daemon_with_ai(
    backend: Arc<dyn AiBackend>,
) -> (std::path::PathBuf, CancellationToken, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("alias.sock");
    let db_path = temp_dir.path().join("history.db");
    let cancel_token = CancellationToken::new();

    let store = HistoryStore::open(&db_path).unwrap();
    let shared_store = Arc::new(Mutex::new(store));

    let state = DaemonState {
        store: shared_store,
        ai_backend: Some(backend),
        cancel_token: cancel_token.clone(),
        enabled: Arc::new(AtomicBool::new(true)),
    };

    let server_path = socket_path.clone();
    tokio::spawn(async move {
        server::run_server(&server_path, state).await.unwrap();
    });

    // Wait for server to bind the socket
    tokio::time::sleep(Duration::from_millis(100)).await;

    (socket_path, cancel_token, temp_dir)
}

/// Send an NDJSON line and read back the response, parsed as a Response.
async fn send_ndjson(stream: &mut UnixStream, request_json: &str) -> Response {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    let mut line = request_json.to_string();
    if !line.ends_with('\n') {
        line.push('\n');
    }
    writer.write_all(line.as_bytes()).await.unwrap();

    let mut response_line = String::new();
    buf_reader.read_line(&mut response_line).await.unwrap();
    serde_json::from_str(&response_line).expect("valid NDJSON response")
}

#[tokio::test]
async fn test_complete_request_returns_suggestion() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // Record a command first so the store has data
        let record_response = send_ndjson(
            &mut stream,
            r#"{"id":"rec1","type":"record","cmd":"git checkout main","cwd":"/home"}"#,
        )
        .await;
        assert_eq!(
            record_response,
            Response::Ack {
                id: "rec1".to_string()
            }
        );

        // Now query for a prefix match
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"r1","type":"complete","buf":"git ch","cur":6}"#,
        )
        .await;

        match response {
            Response::Suggestion { id, text } => {
                assert_eq!(id, "r1");
                assert_eq!(text, "eckout main");
            }
            other => panic!("expected Suggestion, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_record_returns_ack() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"r1","type":"record","cmd":"ls -la","cwd":"/tmp"}"#,
        )
        .await;

        match response {
            Response::Ack { id } => {
                assert_eq!(id, "r1");
            }
            other => panic!("expected Ack, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_complete_empty_db_returns_empty() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"r1","type":"complete","buf":"git ch","cur":6}"#,
        )
        .await;

        match response {
            Response::Suggestion { id, text } => {
                assert_eq!(id, "r1");
                assert_eq!(text, "", "empty DB should return empty suggestion");
            }
            other => panic!("expected Suggestion, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_ping_returns_pong() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"r0","type":"ping"}"#,
        )
        .await;

        match response {
            Response::Pong { id, v } => {
                assert_eq!(id, "r0");
                assert_eq!(v, "0.1.0");
            }
            other => panic!("expected Pong, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_multiple_clients() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        // Client 1: record a command
        {
            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
            let response = send_ndjson(
                &mut stream,
                r#"{"id":"rec1","type":"record","cmd":"docker compose up","cwd":"/project"}"#,
            )
            .await;
            assert_eq!(
                response,
                Response::Ack {
                    id: "rec1".to_string()
                }
            );
        }
        // Client 1 stream is dropped (disconnected)

        // Client 2: query for a prefix match
        {
            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
            let response = send_ndjson(
                &mut stream,
                r#"{"id":"r2","type":"complete","buf":"docker ","cur":7}"#,
            )
            .await;

            match response {
                Response::Suggestion { id, text } => {
                    assert_eq!(id, "r2");
                    assert_eq!(text, "compose up");
                }
                other => panic!("client 2: expected Suggestion, got {:?}", other),
            }
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_generate_returns_command() {
    let mock_backend = Arc::new(MockAiBackend::new("ls -la"));
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon_with_ai(mock_backend).await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"type":"generate","id":"g1","prompt":"list files"}"#,
        )
        .await;

        match response {
            Response::Command { id, text } => {
                assert_eq!(id, "g1");
                assert_eq!(text, "ls -la");
            }
            other => panic!("expected Command response, got: {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_generate_no_backend_returns_error() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"type":"generate","id":"g2","prompt":"hello"}"#,
        )
        .await;

        match response {
            Response::Error { id, msg } => {
                assert_eq!(id, "g2");
                assert!(
                    msg.contains("No AI model configured"),
                    "expected model error, got: {}",
                    msg
                );
            }
            other => panic!("expected Error response, got: {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

// --- New context-aware E2E tests ---

#[tokio::test]
async fn test_record_with_exit_code() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"rec1","type":"record","cmd":"ls","cwd":"/home","exit_code":0}"#,
        )
        .await;
        assert_eq!(response, Response::Ack { id: "rec1".to_string() });
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_record_without_exit_code_backward_compat() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"rec1","type":"record","cmd":"ls","cwd":"/home"}"#,
        )
        .await;
        assert_eq!(response, Response::Ack { id: "rec1".to_string() });
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_complete_with_context_frecency_ranked() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // Record "git status" 5x in /proj with success
        for i in 0..5 {
            let record_json = format!(
                r#"{{"id":"rec{}","type":"record","cmd":"git status","cwd":"/proj","exit_code":0}}"#,
                i
            );
            let response = send_ndjson(&mut stream, &record_json).await;
            assert_eq!(response, Response::Ack { id: format!("rec{}", i) });
        }

        // Record "git stash" 1x in /other with success
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"rec5","type":"record","cmd":"git stash","cwd":"/other","exit_code":0}"#,
        )
        .await;
        assert_eq!(response, Response::Ack { id: "rec5".to_string() });

        // Complete with cwd=/proj context -- should rank "git status" higher
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"c1","type":"complete","buf":"git st","cur":6,"cwd":"/proj"}"#,
        )
        .await;

        match response {
            Response::Suggestion { id, text } => {
                assert_eq!(id, "c1");
                assert_eq!(text, "atus", "expected 'git status' suffix");
            }
            other => panic!("expected Suggestion, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_complete_without_context_backward_compat() {
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // Record a command
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"rec1","type":"record","cmd":"git checkout main","cwd":"/home"}"#,
        )
        .await;
        assert_eq!(response, Response::Ack { id: "rec1".to_string() });

        // Complete without context fields (old format)
        let response = send_ndjson(
            &mut stream,
            r#"{"id":"c1","type":"complete","buf":"git ch","cur":6}"#,
        )
        .await;

        match response {
            Response::Suggestion { id, text } => {
                assert_eq!(id, "c1");
                assert_eq!(text, "eckout main");
            }
            other => panic!("expected Suggestion, got {:?}", other),
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn test_generate_with_context_enriches_prompt() {
    let mock_backend = MockAiBackend::new("cd /proj && git pull");
    let captured = mock_backend.captured_prompt();
    let mock_arc: Arc<dyn AiBackend> = Arc::new(mock_backend);
    let (socket_path, cancel_token, _temp_dir) = spawn_daemon_with_ai(mock_arc).await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_ndjson(
            &mut stream,
            r#"{"type":"generate","id":"g1","prompt":"pull latest","cwd":"/proj","exit_code":1,"git_branch":"main"}"#,
        )
        .await;

        match response {
            Response::Command { id, text } => {
                assert_eq!(id, "g1");
                assert_eq!(text, "cd /proj && git pull");
            }
            other => panic!("expected Command response, got: {:?}", other),
        }

        // Verify context was included in the prompt
        let prompt = captured.lock().unwrap().clone().unwrap();
        assert!(prompt.contains("Current directory: /proj"), "prompt should contain cwd");
        assert!(prompt.contains("Last command failed with exit code: 1"), "prompt should contain exit code");
        assert!(prompt.contains("Git branch: main"), "prompt should contain git branch");
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}
