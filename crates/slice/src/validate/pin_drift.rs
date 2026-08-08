//! Pin-drift review signals over recorded `base.yaml` pins.
//!
//! Both findings are non-blocking [`DiagnosticKind::Review`] — validate
//! still PASSes, but the operator sees the staleness before build.

use std::path::Path;

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Result;
use project::config::Layout;
use project::plan::{Plan, dir_cid, source_cid};

use crate::Base;

/// Emit pin-drift review findings for one slice.
///
/// No-ops when `base.yaml` is absent (pre-refine). Recomputes live
/// digests; does not rewrite plan pins or `base.yaml`.
///
/// # Errors
///
/// Propagates plan / pin / filesystem failures from digest walks.
pub(super) fn findings(
    layout: Layout<'_>, slice_dir: &Path, name: &str,
) -> Result<Vec<Diagnostic>> {
    if !Base::path(slice_dir).is_file() {
        return Ok(Vec::new());
    }
    let base = Base::load(slice_dir)?;
    let mut findings = Vec::new();

    let live_baseline = dir_cid(&layout.specs_dir())?;
    if live_baseline != base.baseline_spec {
        findings.push(review(
            "slice-base-drifted",
            "baseline-spec pin in base.yaml matches the current .emery/specs/ tree",
            format!(
                "slice `{name}` baseline-spec pin `{}` drifted; live digest is `{live_baseline}` — \
                 re-run `emery slice refine {name}` to refresh pins",
                base.baseline_spec
            ),
        ));
    }

    let plan_path = layout.plan_path();
    if !plan_path.is_file() {
        return Ok(findings);
    }
    let plan = Plan::load(&plan_path)?;
    let Some(entry) = plan.entries.iter().find(|entry| entry.name == name) else {
        return Ok(findings);
    };

    for binding in &entry.sources {
        let key = binding.source();
        let Some(pinned) = base.sources.get(key) else {
            continue;
        };
        let Some(plan_binding) = plan.sources.get(key) else {
            continue;
        };
        let live = source_cid(key, plan_binding, layout.project_dir())?;
        if live != *pinned {
            findings.push(review(
                "slice-evidence-stale",
                "bound source pins in base.yaml match the live source trees Evidence was \
                 extracted from",
                format!(
                    "slice `{name}` source `{key}` pin `{pinned}` drifted; live digest is `{live}` \
                     — re-run `emery slice refine {name}` so Evidence tracks the current source"
                ),
            ));
        }
    }

    Ok(findings)
}

fn review(rule_id: &'static str, title: &str, detail: String) -> Diagnostic {
    Diagnostic::finding(
        rule_id,
        title,
        detail,
        Severity::Suggestion,
        DiagnosticKind::Review,
        DiagnosticSource::Deterministic,
        Artifact::Specs,
        None,
    )
}
