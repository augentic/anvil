//! The target merge-gate seam: dispatch one WIT `merge-phase` gate,
//! check the report's slice binding, enforce blocking findings, and
//! persist the schema-gated report beside the slice.

use std::path::Path;

use artifacts::atomic::bytes_write;
use error::Error;
use project::seam::{self, MergePhase, Target};

use super::super::seam_failure;
use crate::{BuildReport, BuildStatus};

/// Dispatch one target merge gate and enforce its report (preflight
/// path — persist happens after a successful return).
#[tracing::instrument(name = "slice.merge.gate", skip_all, fields(phase = %phase, target = %id))]
pub(super) async fn run_gate<T: Target>(
    targets: &T, id: &str, slice: &str, phase: MergePhase, view: &seam::Workspace,
) -> Result<BuildReport, Error> {
    let report = fetch_gate_report(targets, id, slice, phase, view).await?;
    enforce_gate(&report, phase, slice)?;
    Ok(report)
}

/// Fetch one target merge gate report and check the slice-name match.
/// `view` is the read-only workspace over the slice's built result
/// snapshot both gates read the result code through.
pub(super) async fn fetch_gate_report<T: Target>(
    targets: &T, id: &str, slice: &str, phase: MergePhase, view: &seam::Workspace,
) -> Result<BuildReport, Error> {
    let report = targets
        .merge(id.to_string(), slice.to_string(), phase, view.clone())
        .await
        .map_err(|err| seam_failure("merge", id, &err))?;

    if report.slice != slice {
        return Err(Error::validation_failed(
            "target-merge-report-slice-mismatch",
            "the merge gate report's slice matches the slice being merged",
            format!("report names slice `{}`, but the merge ran for `{slice}`", report.slice),
        ));
    }
    Ok(report)
}

/// Enforce blocking findings and report status for one merge gate.
pub(super) fn enforce_gate(
    report: &BuildReport, phase: MergePhase, slice: &str,
) -> Result<(), Error> {
    report.enforce_no_blocking()?;
    if report.status == BuildStatus::Failure {
        return Err(Error::Diag {
            code: match phase {
                MergePhase::Preflight => "target-merge-preflight-failed",
                MergePhase::Postflight => "target-merge-postflight-failed",
            },
            detail: format!(
                "target `{}` reported a failed {phase} merge gate for slice `{slice}` ({} \
                 finding(s))",
                report.target,
                report.findings.len()
            ),
        });
    }
    Ok(())
}

/// Persist one gate's report to `<dir>/<phase>.yaml`, so the archived
/// slice carries both gate outcomes — including a postflight
/// `status: failure` report written before the terminal error returns.
pub(super) fn persist_gate_report(
    dir: &Path, phase: MergePhase, report: &BuildReport,
) -> Result<(), Error> {
    std::fs::create_dir_all(dir).map_err(Error::Io)?;
    let yaml = project::fs::yaml(report)?;
    bytes_write(&dir.join(format!("{phase}.yaml")), yaml.as_bytes())
}
