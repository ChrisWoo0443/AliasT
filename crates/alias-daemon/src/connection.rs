use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio_util::sync::CancellationToken;

use alias_core::ai::AiBackend;
use alias_core::history::HistoryStore;
use alias_protocol::{Request, Response};

/// Handles a single client connection, reading NDJSON requests and writing responses.
///
/// Each line is parsed as a JSON `Request`. The handler dispatches to the
/// appropriate logic (ping, complete, record) and writes back a JSON `Response` line.
/// Exits when the connection is closed or the cancellation token fires.
pub async fn handle_connection(
    stream: UnixStream,
    cancel_token: CancellationToken,
    store: Arc<Mutex<HistoryStore>>,
    ai_backend: Option<Arc<dyn AiBackend>>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line_buffer = String::new();

    tracing::debug!("Connection opened");

    loop {
        line_buffer.clear();

        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::debug!("Connection cancelled by shutdown");
                break;
            }
            bytes_read = buf_reader.read_line(&mut line_buffer) => {
                let bytes_read = bytes_read?;
                if bytes_read == 0 {
                    // Client disconnected
                    tracing::debug!("Connection closed by client");
                    break;
                }

                let trimmed = line_buffer.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<Request>(trimmed) {
                    Ok(request) => dispatch_request(request, &store, &ai_backend).await,
                    Err(parse_error) => Response::Error {
                        id: "unknown".to_string(),
                        msg: parse_error.to_string(),
                    },
                };

                let mut response_json = serde_json::to_string(&response)?;
                response_json.push('\n');
                writer.write_all(response_json.as_bytes()).await?;
            }
        }
    }

    tracing::debug!("Connection handler exiting");
    Ok(())
}

/// Dispatches a parsed request to the appropriate handler.
async fn dispatch_request(
    request: Request,
    store: &Arc<Mutex<HistoryStore>>,
    ai_backend: &Option<Arc<dyn AiBackend>>,
) -> Response {
    match request {
        Request::Ping { id } => Response::Pong {
            id,
            v: env!("CARGO_PKG_VERSION").to_string(),
        },
        Request::Complete { id, buf, cur: _ } => {
            let store_guard = store.lock().unwrap();
            let suggestion_text = alias_core::suggest(&store_guard, &buf).unwrap_or_default();
            Response::Suggestion {
                id,
                text: suggestion_text,
            }
        }
        Request::Record { id, cmd, cwd } => {
            let store_guard = store.lock().unwrap();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            if let Err(err) = store_guard.record_command(&cmd, timestamp, &cwd) {
                tracing::error!("Failed to record command: {err}");
            }
            Response::Ack { id }
        }
        Request::Generate { id, prompt } => match ai_backend {
            Some(backend) => match backend.generate(&prompt).await {
                Ok(command_text) => Response::Command {
                    id,
                    text: command_text,
                },
                Err(err) => Response::Error {
                    id,
                    msg: err.to_string(),
                },
            },
            None => Response::Error {
                id,
                msg: "No AI model configured. Set ALIAS_NL_MODEL env var.".to_string(),
            },
        },
    }
}
