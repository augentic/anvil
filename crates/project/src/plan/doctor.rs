//! Health diagnostics layered on top of `Plan::validate`:
//! `cycle-in-depends-on` and `orphan-source`. Surfaced through
//! `emery plan validate`.

use std::path::Path;

use diagnostics::Diagnostic;

use super::Plan;

mod cycle;
mod orphan_source;

pub use cycle::detect;

/// Stable code for the cycle-detection diagnostic.
pub const CYCLE: &str = "cycle-in-depends-on";
/// Stable code for the orphan-source diagnostic — top-level
/// `sources:` key declared but unreferenced by any entry.
pub const ORPHAN_SOURCE: &str = "orphan-source";

/// Run every `Plan::validate` check, then layer doctor-only
/// diagnostics on top.
///
/// `slices_dir` forwards to `Plan::validate` so those findings stay
/// bit-identical to `emery plan validate`. Order is stable: validate
/// findings, then cycles and orphan sources.
#[must_use]
pub fn doctor(plan: &Plan, slices_dir: Option<&Path>) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = plan.validate(slices_dir);

    out.extend(detect(&plan.entries));
    out.extend(super::decomposition::contraction(&plan.entries));
    out.extend(orphan_source::detect(plan));

    out
}

/// Advance-time gate subset: the structural `Plan::validate` findings
/// plus dependency cycles.
///
/// This is what the execute loop's per-phase advance must be clean of
/// before advancing an entry. The gate itself stays read-only. Cycle
/// findings always block, so callers gate on `has_blocking` over the
/// returned set.
#[must_use]
pub fn advance_gate(plan: &Plan, slices_dir: &Path) -> Vec<Diagnostic> {
    let mut out = plan.validate(Some(slices_dir));
    out.extend(detect(&plan.entries));
    out
}

/// Author-time gate: the full [`doctor`] sweep against the freshly
/// written plan.
///
/// The post-write check the guest `plan author` orchestration runs
/// before exiting. Identical to the `plan validate` findings.
#[must_use]
pub fn author_gate(plan: &Plan, slices_dir: &Path) -> Vec<Diagnostic> {
    doctor(plan, Some(slices_dir))
}

/// The complete `plan validate` report — the [`doctor`] sweep.
#[must_use]
pub fn full_report(plan: &Plan, slices_dir: &Path) -> Vec<Diagnostic> {
    doctor(plan, Some(slices_dir))
}
