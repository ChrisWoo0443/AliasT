use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio_util::sync::CancellationToken;

use aliast_core::history::SuggestionContext;
use aliast_protocol::{Request, Response};

use crate::DaemonState;

/// Handles a single client connection, reading NDJSON requests and writing responses.
///
/// Each line is parsed as a JSON `Request`. The handler dispatches to the
/// appropriate logic (ping, complete, record) and writes back a JSON `Response` line.
/// Exits when the connection is closed or the cancellation token fires.
pub async fn handle_connection(
    stream: UnixStream,
    cancel_token: CancellationToken,
    state: DaemonState,
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
                    Ok(request) => {
                        // Handle shutdown inline to avoid race condition:
                        // write ShuttingDown ack BEFORE cancelling root token
                        if matches!(&request, Request::Shutdown { .. }) {
                            let id = match request {
                                Request::Shutdown { id } => id,
                                _ => unreachable!(),
                            };
                            let response = Response::ShuttingDown { id };
                            let mut response_json = serde_json::to_string(&response)?;
                            response_json.push('\n');
                            writer.write_all(response_json.as_bytes()).await?;
                            writer.flush().await?;
                            // Cancel ROOT token (from DaemonState, not child_token)
                            state.cancel_token.cancel();
                            break;
                        }
                        dispatch_request(request, &state).await
                    }
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

/// Builds an enriched prompt string by prepending environmental context.
///
/// If context fields are present, they are added as a [Context] block
/// before the user's prompt text.
pub fn enrich_prompt(
    prompt: &str,
    cwd: Option<&str>,
    exit_code: Option<i32>,
    git_branch: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(dir) = cwd {
        parts.push(format!("Current directory: {}", dir));
    }
    if let Some(code) = exit_code {
        if code != 0 {
            parts.push(format!("Last command failed with exit code: {}", code));
        }
    }
    if let Some(branch) = git_branch {
        parts.push(format!("Git branch: {}", branch));
    }
    if parts.is_empty() {
        return prompt.to_string();
    }
    format!("[Context]\n{}\n\n{}", parts.join("\n"), prompt)
}

/// Dispatches a parsed request to the appropriate handler.
async fn dispatch_request(request: Request, state: &DaemonState) -> Response {
    match request {
        Request::Ping { id } => Response::Pong {
            id,
            v: env!("CARGO_PKG_VERSION").to_string(),
        },
        Request::Complete {
            id,
            buf,
            cur: _,
            cwd,
            exit_code,
            git_branch,
        } => {
            if !state.enabled.load(Ordering::Relaxed) {
                return Response::Suggestion { id, text: String::new() };
            }
            let store_guard = state.store.lock().unwrap();
            let context = SuggestionContext {
                cwd,
                exit_code,
                git_branch,
            };
            let suggestion_text =
                aliast_core::suggest(&store_guard, &buf, &context).unwrap_or_default();
            Response::Suggestion {
                id,
                text: suggestion_text,
            }
        }
        Request::Record {
            id,
            cmd,
            cwd,
            exit_code,
        } => {
            let store_guard = state.store.lock().unwrap();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            if let Err(err) = store_guard.record_command(&cmd, timestamp, &cwd, exit_code) {
                tracing::error!("Failed to record command: {err}");
            }
            Response::Ack { id }
        }
        Request::Generate {
            id,
            prompt,
            cwd,
            exit_code,
            git_branch,
        } => {
            if !state.enabled.load(Ordering::Relaxed) {
                return Response::Error {
                    id,
                    msg: "aliast is paused. Run `aliast on` to resume.".to_string(),
                };
            }
            match &state.ai_backend {
                Some(backend) => {
                    let enriched = enrich_prompt(
                        &prompt,
                        cwd.as_deref(),
                        exit_code,
                        git_branch.as_deref(),
                    );
                    match backend.generate(&enriched).await {
                        Ok(command_text) => Response::Command {
                            id,
                            text: command_text,
                        },
                        Err(err) => Response::Error {
                            id,
                            msg: err.to_string(),
                        },
                    }
                }
                None => Response::Error {
                    id,
                    msg: "No AI model configured. Set ALIAST_NL_MODEL env var.".to_string(),
                },
            }
        }
        Request::Shutdown { id } => {
            // Shutdown is handled inline in handle_connection to avoid
            // the race condition where cancel_token.cancel() would race
            // the response write. This arm is unreachable but kept for completeness.
            Response::ShuttingDown { id }
        }
        Request::Enable { id } => {
            state.enabled.store(true, Ordering::Relaxed);
            tracing::info!("suggestions enabled");
            Response::Ack { id }
        }
        Request::Disable { id } => {
            state.enabled.store(false, Ordering::Relaxed);
            tracing::info!("suggestions disabled");
            Response::Ack { id }
        }
        Request::GetStatus { id } => {
            let enabled = state.enabled.load(Ordering::Relaxed);
            Response::Status {
                id,
                enabled,
                version: env!("CARGO_PKG_VERSION").to_string(),
                backend: state.backend_name.clone(),
                model: state.model_name.clone(),
            }
        }
    }
}
