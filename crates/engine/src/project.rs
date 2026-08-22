//! `Project` — the spec generator's `.emery/project.yaml` model:
//! identity, the `emery` pin, and the authored source bindings.
//! Written by `emery init`; read fail-closed by `specify`.

use std::path::{Path, PathBuf};

use emery_error::Error;
use omnia_guest::StateStore;
use serde::{Deserialize, Serialize};

use crate::storage;

/// The keyvalue entry carrying the project record.
pub const PROJECT_KEY: &str = "project.yaml";

/// In-memory representation of the spec generator's `project.yaml`.
///
/// `deny_unknown_fields`: the file is machine-written; unknown keys
/// fail the load rather than being silently ignored — pre-1.0 a
/// shape change means re-init.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Project {
    /// Project name (defaults to the project directory name at init).
    pub name: String,

    /// Free-text project description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Minimum `emery` CLI version required to operate on this
    /// project, written by `emery init` as the running binary's
    /// version and enforced by [`Project::load`].
    #[serde(rename = "emery", default, skip_serializing_if = "Option::is_none")]
    pub emery_version: Option<String>,

    /// The authored source bindings `emery specify` extracts from.
    pub sources: Vec<SourceBinding>,
}

/// One authored source binding: a key, the adapter that extracts it,
/// and its content.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SourceBinding {
    /// Stable binding key (the resolved adapter name at init time).
    pub key: String,
    /// The persisted adapter selector (a bare name stays bare; a local
    /// component records its canonical `file://` form).
    pub adapter: String,
    /// What the adapter extracts.
    #[serde(flatten)]
    pub content: BindingContent,
}

/// A binding's content: a read-only workspace view or an inline value.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingContent {
    /// Project-relative root of a read-only source view (`.` binds
    /// the project directory itself).
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

    /// Load and validate the project record from the state store,
    /// enforcing the `emery` pin.
    ///
    /// # Errors
    ///
    /// [`Error::NotInitialized`] when the entry is absent; YAML errors
    /// when it does not parse as this shape (a v1-shaped record
    /// included); [`Error::CliTooOld`] when the pin outruns this
    /// binary.
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

    /// Write this project to the state store, as the same YAML bytes
    /// the pre-seam file writer produced.
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

// Returns `true` when `current < required` under semver ordering.
// Unparseable versions are treated as "not older" — a typo in the pin
// must not brick the project.
fn version_is_older(current: &str, required: &str) -> bool {
    let (Ok(cur), Ok(req)) = (semver::Version::parse(current), semver::Version::parse(required))
    else {
        return false;
    };
    cur < req
}
