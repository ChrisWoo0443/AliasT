use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::net::UnixListener;
use tokio_util::sync::CancellationToken;

use alias_core::history::HistoryStore;

use crate::connection::handle_connection;
use crate::lifecycle;

/// Runs the daemon server, listening for connections on the given Unix socket path.
///
/// Cleans up any stale socket file before binding, then enters an accept loop.
/// Each accepted connection is spawned as a separate tokio task. The server
/// exits when the cancellation token is triggered, cleaning up the socket file.
pub async fn run_server(
    socket_path: &Path,
    cancel_token: CancellationToken,
    store: Arc<Mutex<HistoryStore>>,
) -> Result<()> {
    lifecycle::cleanup_stale_socket(socket_path)?;

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!("Listening on {:?}", socket_path);
    eprintln!("alias-daemon: listening on {}", socket_path.display());

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("Shutdown signal received, stopping server");
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let child_token = cancel_token.child_token();
                        let conn_store = store.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, child_token, conn_store).await {
                                tracing::error!("Connection handler error: {err}");
                            }
                        });
                    }
                    Err(err) => {
                        tracing::error!("Failed to accept connection: {err}");
                    }
                }
            }
        }
    }

    lifecycle::remove_socket(socket_path);
    tracing::info!("Server stopped");
    Ok(())
}
