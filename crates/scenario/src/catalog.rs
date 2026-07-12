//! Embedded catalog of the canonical workflow scenarios.
//!
//! The canonical YAML documents under `quality/scenarios/` are compiled
//! into the crate so downstream harnesses (notably the revision-pinned
//! native harness in `specify-adapters`) consume the scenarios shipped
//! with the engine revision they target instead of carrying copies.

use error::{Error, Result};

use crate::Scenario;

/// One embedded canonical scenario document.
#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    /// Scenario id — the YAML `id` field, which is also the file stem.
    pub id: &'static str,
    /// Raw canonical YAML document.
    pub yaml: &'static str,
}

macro_rules! catalog {
    ($($id:literal),+ $(,)?) => {
        &[$(CatalogEntry {
            id: $id,
            yaml: include_str!(concat!("../../../quality/scenarios/", $id, ".yaml")),
        }),+]
    };
}

/// Every canonical scenario shipped with this engine revision, ordered
/// by id.
pub const CATALOG: &[CatalogEntry] = catalog![
    "composed-init",
    "composed-loop",
    "contract-lifecycle",
    "documentation-multi-slice",
    "documentation-one-slice",
    "execute-fail-resume",
    "execute-pause-resume",
    "guest-execute-loop",
    "intent-only",
    "lead-reconciliation",
    "single-project-plan",
    "target-shape",
    "typescript-multi-slice",
    "workspace-fail-resume",
    "workspace-stale-recovery",
    "workspace-two-projects",
];

/// Parse and validate the embedded canonical scenario `id`.
///
/// # Errors
///
/// Returns `scenario-unknown` when `id` is not in [`CATALOG`], or the
/// same validation errors as [`Scenario::from_yaml`].
pub fn load(id: &str) -> Result<Scenario> {
    let entry = CATALOG.iter().find(|entry| entry.id == id).ok_or_else(|| Error::Diag {
        code: "scenario-unknown",
        detail: format!(
            "no canonical scenario `{id}`; known ids: {}",
            CATALOG.iter().map(|entry| entry.id).collect::<Vec<_>>().join(", ")
        ),
    })?;
    Scenario::from_yaml(entry.yaml)
}
