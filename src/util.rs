use std::path::{Path, PathBuf};

use anyhow::Result;

/// RAII wrapper around a temporary directory that is removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Create (or reclaim) a temp directory with a consistent naming scheme.
    pub fn new(label: &str) -> Result<Self> {
        let dir = std::env::temp_dir().join(format!("alc-{label}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(Self(dir))
    }

    /// Path to the temporary directory.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
