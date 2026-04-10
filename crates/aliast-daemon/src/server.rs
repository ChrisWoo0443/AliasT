use std::path::Path;

use anyhow::Result;
use tokio::net::UnixListener;

use crate::connection::handle_connection;
use crate::lifecycle;
use crate::DaemonState;

/// Runs the daemon server, listening for connections on the given Unix socket path.
///
/// Cleans up any stale socket file before binding, then enters an accept loop.
/// Each accepted connection is spawned as a separate tokio task. The server
/// exits when the cancellation token is triggered, cleaning up the socket file.
pub async fn run_server(socket_path: &Path, state: DaemonState) -> Result<()> {
    lifecycle::cleanup_stale_socket(socket_path)?;

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!("Listening on {:?}", socket_path);
    eprintln!("aliast-daemon: listening on {}", socket_path.display());

    loop {
        tokio::select! {
            _ = state.cancel_token.cancelled() => {
                tracing::info!("Shutdown signal received, stopping server");
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let child_token = state.cancel_token.child_token();
                        let conn_state = state.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, child_token, conn_state).await {
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
