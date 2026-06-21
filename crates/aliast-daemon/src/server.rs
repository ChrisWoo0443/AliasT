use std::path::Path;

use anyhow::Result;
use tokio::net::UnixListener;

use crate::DaemonState;
use crate::connection::handle_connection;
use crate::lifecycle;

/// Cleans up any stale socket and binds a Unix listener at `socket_path`.
///
/// Returns an error if another daemon is already listening or the bind fails,
/// so callers can propagate it (via `?`) and exit immediately instead of
/// spawning a server task whose failure would otherwise go unobserved.
pub fn bind(socket_path: &Path) -> Result<UnixListener> {
    lifecycle::cleanup_stale_socket(socket_path)?;
    let listener = UnixListener::bind(socket_path)?;

    // Restrict the socket to the owner so no other local user can connect (and
    // read suggestions, spend the API budget, or shut the daemon down). Belt and
    // suspenders alongside the 0700 parent directory and the process umask.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600));
    }

    tracing::info!("Listening on {:?}", socket_path);
    eprintln!("aliast: listening on {}", socket_path.display());
    Ok(listener)
}

/// Runs the accept loop on an already-bound listener until cancellation.
///
/// Each accepted connection is spawned as a separate tokio task. The server
/// exits when the cancellation token is triggered, removing the socket file.
pub async fn run_server_with_listener(
    listener: UnixListener,
    socket_path: &Path,
    state: DaemonState,
) -> Result<()> {
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
                        // Back off briefly so a persistent accept error (e.g. fd
                        // exhaustion) does not spin the loop at 100% CPU.
                        tracing::error!("Failed to accept connection: {err}");
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
            }
        }
    }

    lifecycle::remove_socket(socket_path);
    tracing::info!("Server stopped");
    Ok(())
}

/// Binds the socket and runs the accept loop until cancellation.
///
/// Convenience wrapper over [`bind`] + [`run_server_with_listener`]. Prefer
/// calling those two directly when you need to observe bind errors before
/// entering the async loop (e.g. to exit the process non-zero on a failed bind).
pub async fn run_server(socket_path: &Path, state: DaemonState) -> Result<()> {
    let listener = bind(socket_path)?;
    run_server_with_listener(listener, socket_path, state).await
}
