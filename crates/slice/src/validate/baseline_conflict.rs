//! Baseline-drift review signals: touched baselines modified after
//! this slice's `defined_at` (the retired `slice merge
//! --conflict-check` probe, folded into validate).

use std::path::Path;

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Result;
use project::config::Layout;

use crate::merge::{artifact_classes, conflict_check};

/// Emit one non-blocking review finding per baseline modified after
/// the slice's `defined_at`. No-ops before the slice records
/// `defined_at`.
///
/// # Errors
///
/// Propagates metadata / filesystem failures from the drift walk.
pub(super) fn findings(layout: Layout<'_>, slice_dir: &Path) -> Result<Vec<Diagnostic>> {
    let classes = artifact_classes(layout.project_dir(), slice_dir);
    let conflicts = conflict_check(slice_dir, &classes)?;
    Ok(conflicts
        .into_iter()
        .map(|conflict| {
            Diagnostic::finding(
                "slice-baseline-conflict",
                "touched baselines are unmodified since this slice's defined_at",
                format!(
                    "baseline `{}` was modified {} — after this slice's defined_at ({}); review \
                     the drift before the merge phase commits",
                    conflict.adapter,
                    conflict.baseline_modified_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
                    conflict.defined_at,
                ),
                Severity::Suggestion,
                DiagnosticKind::Review,
                DiagnosticSource::Deterministic,
                Artifact::Specs,
                None,
            )
        })
        .collect())
}
