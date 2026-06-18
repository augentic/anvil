//! Agent-inferred, operator-reviewable component catalog (component
//! catalog contract).
//!
//! The catalog lives at `.specify/design-system/components.yaml` and
//! declares shared UI components that the Vectis target factors into
//! shared code at build time. The catalog is **written by
//! `specify catalog infer --phase bind`** (binding the names the build
//! skill or operator parts supply) and **reviewed by the operator**,
//! who may reject or rename entries. An absent catalog still means "no
//! factoring", so projects without one work exactly as before.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specify_error::{Error, Result};
use specify_model::atomic;

use crate::schema;

/// On-disk path relative to project root.
const CATALOG_REL: &str = ".specify/design-system/components.yaml";

/// On-disk path of the operator-authored parts input, relative to the
/// project root.
const PARTS_REL: &str = ".specify/design-system/parts.yaml";

/// Closed status enum for catalog entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentStatus {
    /// The build should factor this as a shared component.
    Confirmed,
    /// The operator has decided this is not a real shared component;
    /// suppresses `slice-catalog-drift` warnings.
    Rejected,
}

/// A single component catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentEntry {
    /// Whether the component is confirmed for shared factoring or
    /// rejected (suppresses drift warnings).
    pub status: ComponentStatus,
    /// Human-readable note for operators and agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Structural fingerprint (lowercase SHA-256 hex) of the normalised
    /// group skeleton this slug was bound to. Recorded by `bind` so a
    /// later `report` run can echo the bound slug for an already-named
    /// cluster (run-to-run binding stability). `None` for
    /// hand-authored entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// The operator-curated component catalog.
///
/// Validated against `schemas/design-system/components.schema.json` on
/// load. Absent catalogs are represented as `None` at the call site —
/// this struct always represents a successfully loaded and validated
/// catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentsCatalog {
    /// Schema version (currently pinned to `1`).
    pub version: u32,
    /// Map of kebab-case component slugs to their metadata.
    pub components: BTreeMap<String, ComponentEntry>,
}

impl ComponentsCatalog {
    /// Load and validate the catalog from a project root.
    ///
    /// Returns `Ok(None)` when the catalog file does not exist (opt-in).
    /// Returns `Err` when the file exists but fails YAML parse or schema
    /// validation.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] if the file exists but cannot be read.
    /// - [`Error::Validation`] if the file fails schema validation.
    pub fn load(project_dir: &Path) -> Result<Option<Self>> {
        let path = project_dir.join(CATALOG_REL);
        if !path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        Self::from_yaml(&content, &path).map(Some)
    }

    /// Parse and validate catalog YAML content.
    ///
    /// `source_path` is used only for error messages.
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] if YAML parsing or schema validation fails.
    fn from_yaml(content: &str, source_path: &Path) -> Result<Self> {
        schema::validate_components_yaml(content, source_path)?;
        let catalog: Self = serde_saphyr::from_str(content).map_err(|err| {
            Error::validation_failed(
                "catalog-schema",
                "components.yaml conforms to schemas/design-system/components.schema.json",
                format!("{}: deserialise failed: {err}", source_path.display()),
            )
        })?;
        Ok(catalog)
    }

    /// Return the path where the catalog lives relative to a project root.
    #[must_use]
    pub fn path_in(project_dir: &Path) -> PathBuf {
        project_dir.join(CATALOG_REL)
    }

    /// Look up the status of a component by slug.
    #[must_use]
    pub fn status_of(&self, slug: &str) -> Option<ComponentStatus> {
        self.components.get(slug).map(|entry| entry.status)
    }

    /// An empty, version-pinned catalog — the starting point when
    /// `specify catalog infer --phase bind` finds no existing file.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            version: 1,
            components: BTreeMap::new(),
        }
    }

    /// Bind `slug` as a `confirmed` component anchored to `fingerprint`,
    /// honouring the no-overwrite rule: an entry that already
    /// exists (whether `confirmed` or `rejected`) is left untouched, so a
    /// `rejected` slug is never silently re-confirmed and a hand-edited
    /// `description` survives. Only a brand-new slug is added. The stored
    /// `fingerprint` is what lets a later `report` run echo this slug as
    /// the cluster's `bound_slug` (run-to-run binding stability).
    pub fn upsert_bound(&mut self, slug: &str, fingerprint: &str, description: Option<String>) {
        if self.components.contains_key(slug) {
            return;
        }
        self.components.insert(
            slug.to_string(),
            ComponentEntry {
                status: ComponentStatus::Confirmed,
                description,
                fingerprint: Some(fingerprint.to_string()),
            },
        );
    }

    /// Reverse `fingerprint → slug` index over the catalog, built from the
    /// `fingerprint` recorded on each entry. Drives `report`'s `bound_slug`
    /// echo so an already-named cluster surfaces its bound slug on a later
    /// run (run-to-run stability). Entries without a stored
    /// fingerprint (hand-authored) contribute nothing.
    #[must_use]
    pub fn fingerprint_index(&self) -> BTreeMap<&str, &str> {
        self.components
            .iter()
            .filter_map(|(slug, entry)| entry.fingerprint.as_deref().map(|fp| (fp, slug.as_str())))
            .collect()
    }

    /// Atomically write the catalog to `.specify/design-system/components.yaml`
    /// under `project_dir`, creating the parent directory when absent.
    ///
    /// # Errors
    ///
    /// - [`Error::YamlSer`] if serialisation fails.
    /// - [`Error::Io`] if the temp-file write or atomic rename fails.
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        atomic::yaml_write(&Self::path_in(project_dir), self)
    }
}

/// A single operator-authored part: an authoritative composition
/// `group` fragment plus optional metadata.
///
/// The `group` value is carried as an opaque [`Value`] — the host never
/// fingerprints it (that is the vectis tool's single normaliser);
/// it only forwards the part file to the tool and reads back each
/// part's slug + `description` to project matched pins into the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Part {
    /// Schema-compliant composition `group` fragment whose normalised
    /// skeleton is the part's identity.
    pub group: Value,
    /// Optional operator note carried into the resolved catalog entry.
    #[serde(default)]
    pub description: Option<String>,
}

/// The operator-authored parts input (`.specify/design-system/parts.yaml`).
///
/// `parts.yaml` is an **input**, hand-authored and owned by the
/// operator beside `tokens.yaml` / `assets.yaml`; the agent-written
/// [`ComponentsCatalog`] is the **resolved** catalog. It is
/// schema-validated on load with no further coherence gate. Absent
/// parts are represented as `None` at the call site.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Parts {
    /// Schema version (currently pinned to `1`).
    pub version: u32,
    /// Map of kebab-case part slugs to their structural payload.
    pub parts: BTreeMap<String, Part>,
}

impl Parts {
    /// Load and validate `parts.yaml` from a project root.
    ///
    /// Returns `Ok(None)` when the file does not exist (an absent parts
    /// input preserves the Part B behaviour exactly — inference with no
    /// pins). Returns `Err` when the file exists but fails YAML parse or
    /// schema validation.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] if the file exists but cannot be read.
    /// - [`Error::Validation`] if the file fails schema validation.
    pub fn load(project_dir: &Path) -> Result<Option<Self>> {
        let path = project_dir.join(PARTS_REL);
        if !path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        Self::from_yaml(&content, &path).map(Some)
    }

    /// Parse and validate parts YAML content.
    ///
    /// `source_path` is used only for error messages.
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] if YAML parsing or schema validation fails.
    fn from_yaml(content: &str, source_path: &Path) -> Result<Self> {
        schema::validate_parts_yaml(content, source_path)?;
        serde_saphyr::from_str(content).map_err(|err| {
            Error::validation_failed(
                "parts-schema",
                "parts.yaml conforms to schemas/design-system/parts.schema.json",
                format!("{}: deserialise failed: {err}", source_path.display()),
            )
        })
    }

    /// Return the path where the parts input lives relative to a project
    /// root. Used to decide whether to forward `--parts` to the tool.
    #[must_use]
    pub fn path_in(project_dir: &Path) -> PathBuf {
        project_dir.join(PARTS_REL)
    }

    /// The operator description for `slug`, if any — carried into the
    /// projected `confirmed` catalog entry.
    #[must_use]
    pub fn description_of(&self, slug: &str) -> Option<&str> {
        self.parts.get(slug).and_then(|part| part.description.as_deref())
    }
}

#[cfg(test)]
mod tests;
