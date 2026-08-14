//! `discovery.yaml` — reviewed-handoff identities plus pinned delivery topology.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::atomic::yaml_write;
use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use super::model::{DefinitionIdentity, SourceBinding, TargetBinding};
use crate::snapshot::SnapshotId;

/// Wire version stamped into every discovery document.
pub const VERSION: u32 = 1;

/// Pinned delivery topology written by the wave-binding phase.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Discovery {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Reviewed-handoff identity.
    pub definition: DefinitionIdentity,
    /// Pinned targets with exact locators, CIDs, and adapter pins.
    #[serde(default)]
    pub targets: BTreeMap<String, TargetBinding>,
    /// Pinned sources: location rows carry a CID; `intent` is `{ adapter, value }`.
    #[serde(default)]
    pub sources: BTreeMap<String, SourceBinding>,
}

impl Discovery {
    /// Parse YAML, reject unknown fields, and enforce closed invariants.
    ///
    /// # Errors
    ///
    /// `discovery-malformed` on YAML/unknown-field failures;
    /// `discovery-version` on a wire-version mismatch;
    /// source-row xor / `intent` refusals from [`SourceBinding::validate`].
    pub fn parse(text: &str) -> Result<Self, Error> {
        let discovery: Self = serde_saphyr::from_str(text).map_err(|err| Error::Diag {
            code: "discovery-malformed",
            detail: err.to_string(),
        })?;
        discovery.validate()?;
        Ok(discovery)
    }

    /// Load and validate a discovery file.
    ///
    /// # Errors
    ///
    /// Filesystem failures; the same closed-shape errors as [`Self::parse`].
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Canonical YAML bytes (trailing newline, stable field order).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`] as a [`SnapshotId`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Atomic write of the canonical document.
    ///
    /// # Errors
    ///
    /// YAML serialization or filesystem failures.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        yaml_write(path, self)
    }

    /// Enforce version and per-source closed shape.
    ///
    /// # Errors
    ///
    /// Typed `discovery-*` / `source-*` diagnostics listed on [`Self::parse`].
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != VERSION {
            return Err(Error::Diag {
                code: "discovery-version",
                detail: format!("discovery version `{}` is not `{VERSION}`", self.version),
            });
        }
        for (key, source) in &self.sources {
            source.validate(key)?;
        }
        Ok(())
    }
}
