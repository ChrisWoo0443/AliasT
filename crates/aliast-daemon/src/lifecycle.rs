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
/// Checks `XDG_RUNTIME_DIR` first, falls back to `/tmp/aliast-{uid}/aliast/aliast.sock`.
pub fn default_socket_path() -> PathBuf {
    socket_path_for(std::env::var("XDG_RUNTIME_DIR").ok())
}

/// Resolves the socket path from an explicit `XDG_RUNTIME_DIR` value.
///
/// An empty value is treated as unset (mirroring the plugin's `${X:-default}`),
/// so the daemon and plugin agree instead of the daemon binding a relative path.
fn socket_path_for(runtime_dir: Option<String>) -> PathBuf {
    if let Some(dir) = runtime_dir.filter(|value| !value.is_empty()) {
        return PathBuf::from(dir).join("aliast").join("aliast.sock");
    }

    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/aliast-{uid}"))
        .join("aliast")
        .join("aliast.sock")
}

/// Removes the socket file at the given path (best-effort, ignores errors).
pub fn remove_socket(path: &Path) {
    let _ = std::fs::remove_file(path);
    tracing::info!("Socket file removed: {:?}", path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_runtime_dir_falls_back_to_tmp() {
        let path = socket_path_for(Some(String::new()));
        assert!(
            path.to_string_lossy().starts_with("/tmp/aliast-"),
            "empty XDG_RUNTIME_DIR must fall back to /tmp, got {path:?}"
        );
    }

    #[test]
    fn unset_runtime_dir_falls_back_to_tmp() {
        let path = socket_path_for(None);
        assert!(
            path.to_string_lossy().starts_with("/tmp/aliast-"),
            "got {path:?}"
        );
    }

    #[test]
    fn set_runtime_dir_is_used() {
        let path = socket_path_for(Some("/run/user/501".to_string()));
        assert_eq!(path, PathBuf::from("/run/user/501/aliast/aliast.sock"));
    }
}
