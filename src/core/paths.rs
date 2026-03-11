//! XDG-compliant path resolution for schemas, config, and project roots.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const DATA_SUBDIR: &str = "openspec";
const PROJECT_DIR: &str = "openspec";

/// Resolved paths for the `OpenSpec` data directory.
#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// Resolve the global data directory (`~/.local/share/openspec` on Linux/macOS).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform data directory cannot be determined.
    pub fn resolve() -> Result<Self> {
        let base = dirs::data_dir().context("unable to determine platform data directory")?;
        Ok(Self {
            root: base.join(DATA_SUBDIR),
        })
    }

    /// Path to the global schemas directory.
    #[must_use]
    pub fn schemas_dir(&self) -> PathBuf {
        self.root.join("schemas")
    }

    /// Path to a specific schema directory within the global store.
    #[must_use]
    pub fn schema_dir(&self, name: &str) -> PathBuf {
        self.schemas_dir().join(name)
    }

    /// Root of the data directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Ensure the data directory tree exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.schemas_dir())
            .with_context(|| format!("creating data directory at {}", self.root.display()))
    }
}

/// Resolved paths for a project's `openspec/` directory.
#[derive(Debug, Clone)]
pub struct ProjectDir {
    root: PathBuf,
}

impl ProjectDir {
    /// Construct from an explicit project root (the parent of `openspec/`).
    #[must_use]
    pub fn from_root(project_root: &Path) -> Self {
        Self {
            root: project_root.join(PROJECT_DIR),
        }
    }

    /// Locate the project's `openspec/` directory by searching upward from `start`.
    ///
    /// # Errors
    ///
    /// Returns an error if no `openspec/` directory is found in the path hierarchy.
    pub fn discover(start: &Path) -> Result<Self> {
        let mut current = start.to_path_buf();
        loop {
            let candidate = current.join(PROJECT_DIR);
            if candidate.is_dir() {
                return Ok(Self { root: candidate });
            }
            if !current.pop() {
                bail!("no openspec/ directory found (searched upward from {})", start.display());
            }
        }
    }

    /// Root of the openspec directory (`<project>/openspec/`).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to `openspec/config.yaml`.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.yaml")
    }

    /// Path to the project's schemas directory.
    #[must_use]
    pub fn schemas_dir(&self) -> PathBuf {
        self.root.join("schemas")
    }

    /// Path to a specific schema within the project.
    #[must_use]
    pub fn schema_dir(&self, name: &str) -> PathBuf {
        self.schemas_dir().join(name)
    }

    /// Path to the changes directory.
    #[must_use]
    pub fn changes_dir(&self) -> PathBuf {
        self.root.join("changes")
    }

    /// Path to a specific change directory.
    #[must_use]
    pub fn change_dir(&self, name: &str) -> PathBuf {
        self.changes_dir().join(name)
    }

    /// Whether the openspec directory already exists.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.root.is_dir()
    }

    /// Ensure the full directory skeleton exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.changes_dir())
            .with_context(|| format!("creating project directory at {}", self.root.display()))
    }
}
