//! Health diagnostics layered on top of `Plan::validate`:
//! `cycle-in-depends-on`, `orphan-source`,
//! `stale-workspace-clone`, and `plan-bootstrap-app-icon-missing`.
//! Surfaced through `specify plan validate`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specify_diagnostics::Diagnostic;

use super::core::Plan;
use crate::registry::Registry;

mod bootstrap_app_icon;
mod cycle;
mod orphan_source;
mod stale_clone;

pub use cycle::detect;

#[cfg(test)]
mod tests;

/// Stable code for the cycle-detection diagnostic.
pub const CYCLE: &str = "cycle-in-depends-on";
/// Stable code for the orphan-source diagnostic — top-level
/// `sources:` key declared but unreferenced by any entry.
pub const ORPHAN_SOURCE: &str = "orphan-source";
/// Stable code for the stale-workspace-clone diagnostic. See
/// [`StaleReason`] for the two ways a clone is classified stale.
pub const STALE_CLONE: &str = "stale-workspace-clone";
/// Stable code for the bootstrap `app-icon` gate (RFC-46 §6.2).
pub const BOOTSTRAP_APP_ICON_MISSING: &str = bootstrap_app_icon::BOOTSTRAP_APP_ICON_MISSING;

/// Why a workspace clone is classified stale by [`STALE_CLONE`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StaleReason {
    /// A remote-backed clone's `origin` differs from the registry URL.
    SignatureChanged,
    /// Slot materialisation does not match the registry URL class or target.
    SlotMismatch,
}

/// Snapshot of the registry or slot signature for staleness comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CloneSignature {
    /// Materialisation kind (`git-clone`, `symlink`, or `other`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_kind: Option<String>,
    /// Repo URL — registry's `url` for the expected signature; git
    /// `origin` for observed remote-backed slots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Adapter identifier from the registry's `adapter` field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Canonical filesystem target for symlink-backed slots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Run every `Plan::validate` check, then layer doctor-only
/// diagnostics on top.
///
/// `slices_dir` and `registry` are forwarded to `Plan::validate` so
/// the validate-level findings are bit-identical to those emitted by
/// `specify plan validate`. `project_dir` is consulted by the
/// stale-workspace-clone and bootstrap `app-icon` checks; pass `None`
/// to skip both (`Plan::doctor_pure` does the same — see the unit tests).
///
/// Every check already emits the neutral [`Diagnostic`] currency, so
/// the validate-level findings pass through unchanged and the health
/// checks append their structured-evidence findings after them.
///
/// The order in the returned vector is stable:
///
///   1. Every `Plan::validate` finding, in the existing order.
///   2. Cycle diagnostics (one per cycle, deduplicated by node-set).
///   3. Orphan source diagnostics (sorted by key).
///   4. Stale workspace clone diagnostics (sorted by project name).
///   5. Bootstrap `app-icon` gate diagnostics (one per failing UI platform).
#[must_use]
pub fn doctor(
    plan: &Plan, slices_dir: Option<&Path>, registry: Option<&Registry>, project_dir: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = plan.validate(slices_dir, registry);

    out.extend(detect(&plan.entries));
    out.extend(orphan_source::detect(plan));
    if let (Some(reg), Some(dir)) = (registry, project_dir) {
        out.extend(stale_clone::detect(reg, dir));
    }
    if let Some(dir) = project_dir {
        out.extend(bootstrap_app_icon::detect(dir));
    }

    out
}

/// Bootstrap `app-icon` gate findings for slice-build prepare (RFC §6.2).
#[must_use]
pub fn bootstrap_app_icon_findings(project_dir: &Path) -> Vec<Diagnostic> {
    bootstrap_app_icon::detect(project_dir)
}
