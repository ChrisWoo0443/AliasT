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

/// Path of the marker that records an explicit `aliast stop`.
///
/// While it exists, the zsh plugin will not auto-respawn the daemon (its precmd
/// hook otherwise resurrects a stopped daemon before the next prompt renders).
/// `aliast start` removes it. Living next to the socket, it is naturally
/// cleared on reboot.
pub fn autostart_marker_path(socket_path: &Path) -> PathBuf {
    socket_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("autostart-disabled")
}

/// Returns true when an explicit stop has disabled daemon auto-start.
pub fn autostart_disabled(socket_path: &Path) -> bool {
    autostart_marker_path(socket_path).exists()
}

/// Records an explicit stop so the plugin stops auto-respawning the daemon.
/// Creates the parent directory if needed.
pub fn disable_autostart(socket_path: &Path) -> std::io::Result<()> {
    let marker = autostart_marker_path(socket_path);
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&marker, b"")
}

/// Clears the explicit-stop marker (best-effort), re-enabling auto-start.
pub fn enable_autostart(socket_path: &Path) {
    let _ = std::fs::remove_file(autostart_marker_path(socket_path));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_autostart_creates_marker_and_enable_removes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("aliast").join("aliast.sock");
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();

        assert!(!autostart_disabled(&socket), "fresh dir: no marker");

        disable_autostart(&socket).unwrap();
        assert!(autostart_disabled(&socket), "marker set after disable");

        enable_autostart(&socket);
        assert!(!autostart_disabled(&socket), "marker cleared after enable");
    }

    #[test]
    fn disable_autostart_creates_parent_dir_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("nonexistent").join("aliast.sock");

        disable_autostart(&socket).unwrap();
        assert!(autostart_disabled(&socket));
    }

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
