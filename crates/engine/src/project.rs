//! The `.emery/project.yaml` model.

use std::path::{Path, PathBuf};

use emery_error::Error;
use omnia_guest::StateStore;
use serde::{Deserialize, Serialize};

use crate::storage;

/// Keyvalue entry carrying the project record.
pub const PROJECT_KEY: &str = "project.yaml";

/// A project's identity, Emery pin, and source bindings.
///
/// Unknown fields fail closed because the record is machine-written.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Project {
    /// Project name.
    pub name: String,

    /// Free-text project description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Minimum Emery CLI version, enforced by [`Project::load`].
    #[serde(rename = "emery", default, skip_serializing_if = "Option::is_none")]
    pub emery_version: Option<String>,

    /// The authored source bindings `emery specify` extracts from.
    pub sources: Vec<SourceBinding>,
}

/// An authored source binding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SourceBinding {
    /// Stable binding key.
    pub key: String,
    /// Persisted adapter selector.
    pub adapter: String,
    /// What the adapter extracts.
    #[serde(flatten)]
    pub content: BindingContent,
}

/// Workspace or inline source content.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingContent {
    /// Project-relative read-only root; `.` binds the project.
    Workspace(String),
    /// Inline value; no filesystem view.
    Value(String),
}

impl Project {
    /// Absolute path of `<project_dir>/.emery/project.yaml`.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        project_dir.join(".emery").join("project.yaml")
    }

    /// Loads the project and enforces its Emery version floor.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotInitialized`], YAML errors, or
    /// [`Error::CliTooOld`] as applicable.
    pub async fn load<S: StateStore>(state: &S) -> Result<Self, Error> {
        let bytes = state
            .get(PROJECT_KEY)
            .await
            .map_err(|err| storage::failed("reading the project record", &err))?
            .ok_or(Error::NotInitialized)?;
        let project: Self = serde_saphyr::from_str(&String::from_utf8_lossy(&bytes))?;
        let current = env!("CARGO_PKG_VERSION");
        if let Some(required) = &project.emery_version
            && version_is_older(current, required)
        {
            return Err(Error::CliTooOld {
                required: required.clone(),
                found: current.to_string(),
            });
        }
        Ok(project)
    }

    /// Writes the project record as YAML.
    ///
    /// # Errors
    ///
    /// Propagates serialization and storage failures.
    pub async fn store<S: StateStore>(&self, state: &S) -> Result<(), Error> {
        let body = emery_artifacts::atomic::serialise_yaml(self)?;
        state
            .set(PROJECT_KEY, body.as_bytes(), None)
            .await
            .map_err(|err| storage::failed("writing the project record", &err))?;
        Ok(())
    }
}

// Unparseable versions are permissive so a malformed pin cannot brick recovery.
fn version_is_older(current: &str, required: &str) -> bool {
    let (Ok(cur), Ok(req)) = (semver::Version::parse(current), semver::Version::parse(required))
    else {
        return false;
    };
    cur < req
}
