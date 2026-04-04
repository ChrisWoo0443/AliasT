use std::path::Path;

use anyhow::Result;

/// Migrates data files from the old `alias` data directory to the new `aliast` directory.
///
/// Moves `history.db` and `daemon.log` if they exist at the old path and do not
/// already exist at the new path. This is a silent, best-effort operation --
/// existing files at the new location are never overwritten.
pub fn migrate_data_files(old_dir: &Path, new_dir: &Path) -> Result<()> {
    migrate_file(old_dir, new_dir, "history.db")?;
    migrate_file(old_dir, new_dir, "daemon.log")?;
    Ok(())
}

fn migrate_file(old_dir: &Path, new_dir: &Path, filename: &str) -> Result<()> {
    let old_path = old_dir.join(filename);
    let new_path = new_dir.join(filename);
    if old_path.exists() && !new_path.exists() {
        std::fs::create_dir_all(new_dir)?;
        std::fs::rename(&old_path, &new_path)?;
    }
    Ok(())
}
