//! Pin-drift signals over recorded `base.yaml` pins.
//!
//! One digest walk feeds validate's non-blocking review findings and
//! the execute loop's [`pins_drifted`] re-refine staleness probe.
//! Binding-set mismatches (a plan source added or removed after
//! refine) count as drift so execute re-refines instead of building
//! on Evidence that never saw the current source set.

use std::collections::BTreeSet;
use std::path::Path;

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Result;
use project::config::Layout;
use project::plan::{Plan, dir_cid, source_cid};

use crate::Base;

/// One recorded pin that no longer matches the live tree or binding set.
enum Drift {
    /// `base.yaml.baseline_spec` vs the live `.emery/specs/` digest.
    Baseline { pinned: String, live: String },
    /// A `base.yaml.sources` pin vs the live bound source tree.
    Source { key: String, pinned: String, live: String },
    /// Plan entry binds a source with no `base.yaml` pin (added after refine).
    SourceMissing { key: String },
    /// `base.yaml` pins a source the entry no longer binds (removed after refine).
    SourceOrphan { key: String },
}

/// Walk every recorded pin and collect the drifted ones. Empty when
/// `base.yaml` is absent (pre-refine).
fn drifts(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<Vec<Drift>> {
    if !Base::path(slice_dir).is_file() {
        return Ok(Vec::new());
    }
    let base = Base::load(slice_dir)?;
    let mut out = Vec::new();

    let live_baseline = dir_cid(&layout.specs_dir())?;
    if live_baseline != base.baseline_spec {
        out.push(Drift::Baseline {
            pinned: base.baseline_spec.to_string(),
            live: live_baseline.to_string(),
        });
    }

    let plan_path = layout.plan_path();
    if !plan_path.is_file() {
        return Ok(out);
    }
    let plan = Plan::load(&plan_path)?;
    let Some(entry) = plan.entries.iter().find(|entry| entry.name == name) else {
        return Ok(out);
    };

    let bound: BTreeSet<&str> = entry.sources.iter().map(|b| b.source()).collect();
    for key in base.sources.keys() {
        if !bound.contains(key.as_str()) {
            out.push(Drift::SourceOrphan { key: key.clone() });
        }
    }

    for binding in &entry.sources {
        let key = binding.source();
        let Some(pinned) = base.sources.get(key) else {
            out.push(Drift::SourceMissing { key: key.to_string() });
            continue;
        };
        let Some(plan_binding) = plan.sources.get(key) else {
            continue;
        };
        let live = source_cid(key, plan_binding, layout.project_dir())?;
        if live != *pinned {
            out.push(Drift::Source {
                key: key.to_string(),
                pinned: pinned.to_string(),
                live: live.to_string(),
            });
        }
    }

    Ok(out)
}

/// True when any recorded `base.yaml` pin no longer matches the live
/// baseline / source trees, or the entry's source set no longer matches
/// the pinned keys — the execute loop's staleness probe.
///
/// # Errors
///
/// Propagates plan / pin / filesystem failures from digest walks.
pub fn pins_drifted(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<bool> {
    Ok(!drifts(layout, slice_dir, name)?.is_empty())
}

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
    Ok(drifts(layout, slice_dir, name)?
        .into_iter()
        .map(|drift| match drift {
            Drift::Baseline { pinned, live } => review(
                "slice-base-drifted",
                "baseline-spec pin in base.yaml matches the current .emery/specs/ tree",
                format!(
                    "slice `{name}` baseline-spec pin `{pinned}` drifted; live digest is `{live}` \
                     — re-running `emery plan execute` re-refines this slice under the epoch"
                ),
            ),
            Drift::Source { key, pinned, live } => review(
                "slice-evidence-stale",
                "bound source pins in base.yaml match the live source trees Evidence was \
                 extracted from",
                format!(
                    "slice `{name}` source `{key}` pin `{pinned}` drifted; live digest is \
                     `{live}` — re-running `emery plan execute` re-refines this slice so \
                     Evidence tracks the current source"
                ),
            ),
            Drift::SourceMissing { key } => review(
                "slice-evidence-stale",
                "bound source pins in base.yaml match the live source trees Evidence was \
                 extracted from",
                format!(
                    "slice `{name}` binds source `{key}` with no base.yaml pin — re-running \
                     `emery plan execute` re-refines this slice so Evidence covers the \
                     current source set"
                ),
            ),
            Drift::SourceOrphan { key } => review(
                "slice-evidence-stale",
                "bound source pins in base.yaml match the live source trees Evidence was \
                 extracted from",
                format!(
                    "slice `{name}` base.yaml pins source `{key}` which the entry no longer \
                     binds — re-running `emery plan execute` re-refines this slice so \
                     Evidence matches the current source set"
                ),
            ),
        })
        .collect())
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
