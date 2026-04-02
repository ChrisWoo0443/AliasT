use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Cleans up a stale Unix domain socket file at the given path.
///
/// If the socket file exists and a daemon is already listening on it,
/// returns an error. If the file exists but no daemon is listening,
/// removes the stale file. Creates parent directories with 0o700
/// permissions if they do not exist.
pub fn cleanup_stale_socket(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        // Try connecting to see if a daemon is already running
        match std::os::unix::net::UnixStream::connect(path) {
            Ok(_) => {
                anyhow::bail!("Another daemon is already running at {:?}", path);
            }
            Err(_) => {
                tracing::info!("Removing stale socket file: {:?}", path);
                std::fs::remove_file(path)?;
            }
        }
    }

    // Ensure parent directory exists with restrictive permissions
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        let permissions = Permissions::from_mode(0o700);
        std::fs::set_permissions(parent, permissions)?;
    }

    Ok(())
}

/// Returns the default socket path following XDG conventions.
///
/// Checks `XDG_RUNTIME_DIR` first, falls back to `/tmp/alias-{uid}/alias/alias.sock`.
pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("alias").join("alias.sock");
    }

    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/alias-{uid}"))
        .join("alias")
        .join("alias.sock")
}

/// Removes the socket file at the given path (best-effort, ignores errors).
pub fn remove_socket(path: &Path) {
    let _ = std::fs::remove_file(path);
    tracing::info!("Socket file removed: {:?}", path);
}
