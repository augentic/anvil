//! Health diagnostics layered on top of `Plan::validate`:
//! `cycle-in-depends-on`, `orphan-source`, and
//! `stale-workspace-clone`. Surfaced through `specify plan validate`.

use std::path::Path;

use schema::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};

use super::core::Plan;
use crate::registry::Registry;

mod cycle;
mod orphan_source;
mod stale_clone;

pub use cycle::detect;

/// Stable code for the cycle-detection diagnostic.
pub const CYCLE: &str = "cycle-in-depends-on";
/// Stable code for the orphan-source diagnostic — top-level
/// `sources:` key declared but unreferenced by any entry.
pub const ORPHAN_SOURCE: &str = "orphan-source";
/// Stable code for the stale-workspace-clone diagnostic. See
/// [`StaleReason`] for the two ways a clone is classified stale.
pub const STALE_CLONE: &str = "stale-workspace-clone";

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
/// stale-workspace-clone check; pass `None` to skip it.
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

    out
}

/// Claim-time gate subset: the structural `Plan::validate` findings
/// plus dependency cycles — what `plan next` (and the execute loop's
/// per-phase claim) must be clean of before advancing an entry.
/// Deliberately registry-free: claiming works in registry-less projects
/// and must stay read-only. Cycle findings always block, so callers
/// gate on `blocking_present` over the returned set.
#[must_use]
pub fn claim_gate(plan: &Plan, slices_dir: &Path) -> Vec<Diagnostic> {
    let mut out = plan.validate(Some(slices_dir), None);
    out.extend(detect(&plan.entries));
    out
}

/// Author-time gate: the full [`doctor`] sweep against the freshly
/// written plan — the post-write check the guest `plan author`
/// orchestration runs before exiting at `pending`. Identical to the
/// `plan validate` findings minus the verb-only registry-shape and
/// topology-cache staleness surfaces (which need the verb's provider).
///
/// # Errors
///
/// Propagates the [`Registry`] load failure — an unreadable registry
/// aborts authoring rather than silently skipping the cross-registry
/// checks.
pub fn author_gate(
    plan: &Plan, slices_dir: &Path, project_dir: &Path,
) -> error::Result<Vec<Diagnostic>> {
    let registry = Registry::load(project_dir)?;
    Ok(doctor(plan, Some(slices_dir), registry.as_ref(), Some(project_dir)))
}

/// The complete `plan validate` report: the [`doctor`] sweep plus the
/// verb-only surfaces — the `registry-shape` finding when the registry
/// fails to load, and the workspace topology-cache staleness findings
/// when it loads. Finding order is stable: doctor findings first, then
/// `registry-shape`, then staleness.
pub fn full_report(
    resolver: &impl crate::adapter::Resolver, plan: &Plan, layout: crate::config::Layout<'_>,
) -> Vec<Diagnostic> {
    use schema::diagnostics::Severity;

    use crate::change::plan::core::validate::plan_finding;

    let project_dir = layout.project_dir();
    let (registry, registry_err) = match Registry::load(project_dir) {
        Ok(reg) => (reg, None),
        Err(err) => (None, Some(err)),
    };
    let mut results =
        doctor(plan, Some(&layout.slices_dir()), registry.as_ref(), Some(project_dir));
    if let Some(err) = registry_err {
        results.push(plan_finding("registry-shape", Severity::Important, err.to_string(), None));
    }
    if let Some(reg) = &registry {
        results.extend(crate::registry::cache_staleness(
            resolver,
            reg,
            &project_dir.join("workspace"),
            &layout.topology_lock_path(),
        ));
    }
    results
}
