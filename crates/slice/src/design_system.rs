//! Agent-inferred, operator-reviewable component catalog (component
//! catalog contract).
//!
//! The catalog lives at `.emery/design-system/components.yaml` and
//! declares shared UI components that the Vectis target factors into
//! shared code at build time. The catalog is **written by the guest
//! build orchestration's bind bookkeeping** (driven by the
//! `${SLICE_DIR}/build/component-bindings.yaml` artifact the target
//! build emits) and **reviewed by the operator**, who may reject or
//! rename entries. An absent catalog still means "no factoring", so
//! projects without one work exactly as before.

use std::collections::BTreeMap;
use std::path::Path;

use error::{Error, Result};
use serde::{Deserialize, Serialize};

/// On-disk path relative to project root.
const CATALOG_REL: &str = ".emery/design-system/components.yaml";

/// Shared load kernel for the design-system YAML inputs.
///
/// Reads `<project_dir>/<rel>`, returning `Ok(None)` when the file is
/// absent (the inputs are opt-in), then deserialises through the typed
/// shape. `code` / `rule` label the deserialise failure.
fn load_validated<T: serde::de::DeserializeOwned>(
    project_dir: &Path, rel: &str, code: &'static str, rule: &'static str,
) -> Result<Option<T>> {
    let path = project_dir.join(rel);
    if !path.is_file() {
        return Ok(None);
    }
    let content = project::fs::read_text(&path)?;
    let value: T = serde_saphyr::from_str(&content).map_err(|err| {
        Error::validation_failed(
            code,
            rule,
            format!("{}: deserialise failed: {err}", path.display()),
        )
    })?;
    Ok(Some(value))
}

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
/// Absent catalogs are represented as `None` at the call site — this
/// struct always represents a successfully loaded catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentsCatalog {
    /// Schema version (currently pinned to `1`).
    pub version: u32,
    /// Map of kebab-case component slugs to their metadata.
    pub components: BTreeMap<String, ComponentEntry>,
}

impl ComponentsCatalog {
    /// Load the catalog from a project root.
    ///
    /// Returns `Ok(None)` when the catalog file does not exist (opt-in).
    /// Returns `Err` when the file exists but fails the typed parse.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] if the file exists but cannot be read.
    /// - [`Error::Validation`] if the file fails the typed parse.
    pub fn load(project_dir: &Path) -> Result<Option<Self>> {
        load_validated(
            project_dir,
            CATALOG_REL,
            "catalog-schema",
            "components.yaml deserialises as a component catalog",
        )
    }

    /// Look up the status of a component by slug.
    #[must_use]
    pub fn status_of(&self, slug: &str) -> Option<ComponentStatus> {
        self.components.get(slug).map(|entry| entry.status)
    }
}
