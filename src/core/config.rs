//! Project configuration model -- parsed representation of `openspec/config.yaml`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Project-level `OpenSpec` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Schema name to use (must match a directory under `schemas/`).
    pub schema: String,

    /// Free-form project context injected into artifact generation.
    #[serde(default)]
    pub context: String,

    /// Per-artifact rule overrides keyed by artifact ID.
    #[serde(default)]
    pub rules: BTreeMap<String, Vec<String>>,
}

impl ProjectConfig {
    /// Load config from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_yaml::from_str(&contents)
            .with_context(|| format!("parsing config at {}", path.display()))
    }

    /// Serialize and write config to a YAML file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file I/O fails.
    pub fn write(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("serializing config")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        std::fs::write(path, yaml).with_context(|| format!("writing {}", path.display()))
    }

    /// Build a new config with defaults from a schema name and user-supplied context.
    pub fn new(schema: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            context: context.into(),
            rules: BTreeMap::new(),
        }
    }
}
