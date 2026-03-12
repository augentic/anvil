//! Schema model -- parsed representation of a `schema.yaml` file.

use serde::Deserialize;

/// A schema definition loaded from `schema.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    /// Schema name (e.g. "omnia").
    pub name: String,

    /// Schema version number.
    pub version: u32,

    /// Human-readable description.
    pub description: String,

    /// Ordered list of artifact definitions.
    #[serde(default)]
    pub artifacts: Vec<Artifact>,

    /// Apply-phase configuration.
    pub apply: Option<ApplyConfig>,
}

/// A single artifact that the schema defines.
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    /// Artifact identifier (e.g. "proposal", "specs", "design", "tasks").
    pub id: String,

    /// Glob or path pattern this artifact generates.
    pub generates: String,

    /// Human-readable description.
    pub description: String,

    /// Template filename within the schema's `templates/` directory.
    pub template: String,

    /// Instruction text for the artifact (used by agents).
    #[serde(default)]
    pub instruction: String,

    /// IDs of artifacts that must be completed before this one.
    #[serde(default)]
    pub requires: Vec<String>,
}

/// Configuration for the apply phase.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyConfig {
    /// Artifact IDs that must be complete before apply can run.
    #[serde(default)]
    pub requires: Vec<String>,

    /// File to track for progress (e.g. "tasks.md").
    pub tracks: Option<String>,

    /// Instruction text for the apply phase.
    #[serde(default)]
    pub instruction: String,
}

impl Schema {
    /// Parse a schema from YAML bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the YAML is invalid or does not match the schema format.
    pub fn from_yaml(bytes: &[u8]) -> anyhow::Result<Self> {
        serde_yaml::from_slice(bytes).map_err(|e| anyhow::anyhow!("invalid schema.yaml: {e}"))
    }

    /// Load a schema by name from a project's `specify/schemas/<name>/schema.yaml`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(project: &super::paths::ProjectDir, schema_name: &str) -> anyhow::Result<Self> {
        let path = project.schema_dir(schema_name).join("schema.yaml");
        let bytes = std::fs::read(&path).map_err(|e| {
            anyhow::anyhow!("schema '{schema_name}' not found at {}: {e}", path.display())
        })?;
        Self::from_yaml(&bytes)
    }

    /// Return the list of template filenames referenced by artifacts.
    #[must_use]
    pub fn template_names(&self) -> Vec<&str> {
        self.artifacts.iter().map(|a| a.template.as_str()).collect()
    }

    /// Find an artifact by ID.
    #[must_use]
    pub fn artifact(&self, id: &str) -> Option<&Artifact> {
        self.artifacts.iter().find(|a| a.id == id)
    }
}
