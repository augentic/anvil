//! Filesystem helpers shared by the trial and scenario runners.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};

/// Recursively copy the tree at `from` into `to`, creating `to`.
///
/// Symlinks resolve like the build-time prose embed: linked directories
/// and files are copied as real content under the link-name path.
///
/// # Errors
///
/// Returns an error when any directory or file cannot be read or written,
/// including a dangling symlink.
pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        // `metadata` follows symlinks; a dangling link errors here rather
        // than being silently dropped from the copy.
        let metadata = fs::metadata(&path)
            .with_context(|| format!("resolve {} (dangling symlink?)", path.display()))?;
        if metadata.is_dir() {
            copy_tree(&path, &dest)?;
        } else {
            fs::copy(&path, &dest).with_context(|| format!("copying {}", path.display()))?;
        }
    }
    Ok(())
}
