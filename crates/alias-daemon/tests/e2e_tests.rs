//! End-to-end integration tests proving the full NDJSON request/response loop.
//!
//! These tests start a real daemon server on a temp socket, connect a client,
//! send NDJSON requests, and validate the responses -- proving the complete
//! communication loop works over Unix domain sockets.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use alias_daemon::server;
use alias_protocol::Response;

/// Start daemon on a temp socket, returning path, cancel token, and temp dir guard.
async fn spawn_daemon() -> (std::path::PathBuf, CancellationToken, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("alias.sock");
    let cancel_token = CancellationToken::new();

    let server_path = socket_path.clone();
    let server_token = cancel_token.clone();
    tokio::spawn(async move {
        server::run_server(&server_path, server_token).await.unwrap();
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
        // Client 1: send complete request
        {
            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
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
                other => panic!("client 1: expected Suggestion, got {:?}", other),
            }
        }
        // Client 1 stream is dropped (disconnected)

        // Client 2: send ping request
        {
            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
            let response = send_ndjson(
                &mut stream,
                r#"{"id":"r2","type":"ping"}"#,
            )
            .await;

            match response {
                Response::Pong { id, v } => {
                    assert_eq!(id, "r2");
                    assert_eq!(v, "0.1.0");
                }
                other => panic!("client 2: expected Pong, got {:?}", other),
            }
        }
    })
    .await;

    cancel_token.cancel();
    assert!(result.is_ok(), "test timed out");
}
