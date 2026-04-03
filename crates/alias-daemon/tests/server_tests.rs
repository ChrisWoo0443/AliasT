use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use alias_core::history::HistoryStore;
use alias_daemon::server;

/// Helper: start a server on a temp socket with a fresh HistoryStore,
/// returning the path and cancel token.
async fn start_test_server() -> (std::path::PathBuf, CancellationToken, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("test.sock");
    let db_path = temp_dir.path().join("history.db");
    let cancel_token = CancellationToken::new();

    let store = HistoryStore::open(&db_path).unwrap();
    let shared_store = Arc::new(Mutex::new(store));

    let server_path = socket_path.clone();
    let server_token = cancel_token.clone();
    tokio::spawn(async move {
        server::run_server(&server_path, server_token, shared_store)
            .await
            .unwrap();
    });

    // Wait briefly for the server to start listening
    tokio::time::sleep(Duration::from_millis(50)).await;

    (socket_path, cancel_token, temp_dir)
}

/// Helper: send an NDJSON line and read the response line.
async fn send_and_receive(stream: &mut UnixStream, request_json: &str) -> String {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    let mut request_line = request_json.to_string();
    if !request_line.ends_with('\n') {
        request_line.push('\n');
    }
    writer.write_all(request_line.as_bytes()).await.unwrap();

    let mut response_line = String::new();
    buf_reader.read_line(&mut response_line).await.unwrap();
    response_line
}

#[tokio::test]
async fn server_accepts_connection_and_reads_ndjson() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_and_receive(&mut stream, r#"{"type":"ping","id":"t1"}"#).await;
        assert!(!response.is_empty(), "should receive a response");
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn server_responds_to_ping_with_pong() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_and_receive(&mut stream, r#"{"type":"ping","id":"p1"}"#).await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "pong");
        assert_eq!(parsed["id"], "p1");
        assert_eq!(parsed["v"], "0.1.0");
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn server_responds_to_complete_with_suggestion() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // Record a command first so the store has data for prefix match
        let _record_response = send_and_receive(
            &mut stream,
            r#"{"type":"record","id":"r0","cmd":"git checkout main","cwd":"/home"}"#,
        )
        .await;

        let response = send_and_receive(
            &mut stream,
            r#"{"type":"complete","id":"c1","buf":"git ch","cur":6}"#,
        )
        .await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "suggestion");
        assert_eq!(parsed["id"], "c1");
        assert_eq!(parsed["text"], "eckout main");
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn server_responds_with_error_for_malformed_json() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = send_and_receive(&mut stream, "not valid json at all").await;

        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["id"], "unknown");
        assert!(!parsed["msg"].as_str().unwrap().is_empty());
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn server_handles_multiple_sequential_requests() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    let result = timeout(Duration::from_secs(5), async {
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // First request: ping
        let response1 = send_and_receive(&mut stream, r#"{"type":"ping","id":"m1"}"#).await;
        let parsed1: serde_json::Value = serde_json::from_str(&response1).unwrap();
        assert_eq!(parsed1["type"], "pong");
        assert_eq!(parsed1["id"], "m1");

        // Record a command so complete has data
        let _record_response = send_and_receive(
            &mut stream,
            r#"{"type":"record","id":"r0","cmd":"ls -la","cwd":"/tmp"}"#,
        )
        .await;

        // Second request: complete
        let response2 = send_and_receive(
            &mut stream,
            r#"{"type":"complete","id":"m2","buf":"ls","cur":2}"#,
        )
        .await;
        let parsed2: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(parsed2["type"], "suggestion");
        assert_eq!(parsed2["id"], "m2");
        assert_eq!(parsed2["text"], " -la");
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}

#[tokio::test]
async fn server_shuts_down_on_cancellation_token() {
    let (socket_path, cancel_token, _temp_dir) = start_test_server().await;

    // Verify server is running
    let stream = UnixStream::connect(&socket_path).await;
    assert!(stream.is_ok(), "should be able to connect before cancellation");
    drop(stream);

    // Cancel the server
    cancel_token.cancel();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Server should have cleaned up -- socket file should be removed
    // Note: connection might still succeed briefly due to race, so we check socket file
    assert!(
        !socket_path.exists(),
        "socket file should be removed after shutdown"
    );
}
