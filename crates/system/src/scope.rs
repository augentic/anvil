//! `scope.yaml` — the operator-declared system boundary (RFC-104 D2).
//! Names the decision the survey must support; carries no locators.

use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// The one supported `scope.yaml` schema version.
const VERSION: u32 = 1;

/// The declared boundary at `<system>/scope.yaml`.
///
/// Operator-owned; the engine validates on load and never writes it.
/// Exact evidence locators live on `coverage.yaml` rows, not here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scope {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// Stable kebab-case identity of the definition engagement.
    pub id: String,
    /// The investment decision this survey must support.
    pub decision: String,
    /// Products inside the declared boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<String>,
    /// Critical journeys inside the declared boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journeys: Vec<String>,
    /// Deployment environments inside the declared boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<String>,
    /// Owning organizations inside the declared boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organizations: Vec<String>,
}

impl Scope {
    /// Load and validate `scope.yaml` from `path`.
    ///
    /// # Errors
    ///
    /// - `system-scope-missing` when the file is absent (the operator
    ///   creates the declared inputs; there is no `system init`).
    /// - `Error::YamlDe` for malformed YAML or unknown fields.
    /// - `system-scope-invalid` for an unsupported `version` or an
    ///   empty `id` / `decision`.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Diag {
                    code: "system-scope-missing",
                    detail: format!("scope.yaml not found at {}", path.display()),
                });
            }
            Err(err) => return Err(Error::Io(err)),
        };
        let scope: Self = serde_saphyr::from_str(&text)?;
        scope.validate()?;
        Ok(scope)
    }

    /// Content digest of the canonical YAML encoding, independent of
    /// on-disk formatting (the D10 `scope-digest`).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    fn validate(&self) -> Result<(), Error> {
        if self.version != VERSION {
            return Err(Error::validation_failed(
                "system-scope-invalid",
                "unsupported version",
                format!("scope.yaml version {} is not {VERSION}", self.version),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(Error::validation_failed(
                "system-scope-invalid",
                "id required",
                "scope.yaml `id` must be a non-empty identifier".to_string(),
            ));
        }
        if self.decision.trim().is_empty() {
            return Err(Error::validation_failed(
                "system-scope-invalid",
                "decision required",
                "scope.yaml `decision` must state the decision the survey supports".to_string(),
            ));
        }
        Ok(())
    }
}
