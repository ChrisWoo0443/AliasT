use std::path::Path;

use anyhow::Result;
use tokio::net::UnixListener;
use tokio_util::task::TaskTracker;

use crate::DaemonState;
use crate::connection::handle_connection;
use crate::lifecycle;

/// How long shutdown waits for in-flight connections (e.g. a slow NL generate)
/// to finish before the process exits.
const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// How often the daemon verifies its socket file still exists and is its own.
/// macOS periodically purges /tmp: a daemon whose socket file vanished (or was
/// replaced by a plugin-respawned successor at the same path) can never be
/// reached by `aliast stop` again, so it must exit instead of lingering as an
/// orphaned process.
const SOCKET_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// The inode of the file at `path`, or None if it cannot be stat'ed.
fn socket_inode(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).map(|meta| meta.ino()).ok()
}

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
    run_server_with_listener_checked(listener, socket_path, state, SOCKET_CHECK_INTERVAL).await
}

/// Testable variant of [`run_server_with_listener`] with an explicit socket
/// self-check interval.
pub async fn run_server_with_listener_checked(
    listener: UnixListener,
    socket_path: &Path,
    state: DaemonState,
    check_interval: std::time::Duration,
) -> Result<()> {
    let connections = TaskTracker::new();

    // Identity of the socket file this server bound. If the file at the path
    // later stops matching -- deleted by /tmp cleanup, or replaced by a
    // successor daemon -- this process is unreachable by IPC and must exit.
    let bound_inode = socket_inode(socket_path);
    let mut socket_check = tokio::time::interval(check_interval);

    loop {
        tokio::select! {
            _ = state.cancel_token.cancelled() => {
                tracing::info!("Shutdown signal received, stopping server");
                break;
            }
            _ = socket_check.tick() => {
                if bound_inode.is_some() && socket_inode(socket_path) != bound_inode {
                    tracing::warn!(
                        ?socket_path,
                        "socket file missing or replaced -- shutting down instead of lingering as an orphan"
                    );
                    // Cancel the ROOT token so main() stops waiting on signals
                    // and the whole process exits, not just this task.
                    state.cancel_token.cancel();
                    break;
                }
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let child_token = state.cancel_token.child_token();
                        let conn_state = state.clone();
                        connections.spawn(async move {
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

    // Give in-flight connections a bounded grace period to finish (e.g. a slow NL
    // generate) instead of hard-killing them when the process exits.
    connections.close();
    if tokio::time::timeout(DRAIN_TIMEOUT, connections.wait())
        .await
        .is_err()
    {
        tracing::warn!("Timed out draining connections on shutdown");
    }

    // Remove the socket file only if it is still the one this server bound:
    // a successor daemon may own the path by now, and deleting its socket
    // would strand it exactly the way this check exists to prevent.
    if bound_inode.is_some() && socket_inode(socket_path) == bound_inode {
        lifecycle::remove_socket(socket_path);
    }
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
