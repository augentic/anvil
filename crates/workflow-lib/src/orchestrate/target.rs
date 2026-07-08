//! Target-axis build orchestrator.

use std::path::Path;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::Error;
use specify_model::atomic::bytes_write;

use super::{seam_failure, target_adapter_id};
use crate::adapter::BuildInputDeclaration;
use crate::config::Layout;
use crate::init::adapter_ref_from_value;
use crate::journal::{self, EventKind};
use crate::schema::{validate_build_report_json, validate_build_request_json};
use crate::seam::{Input, TargetSeam, WorkingTree};
use crate::slice::{
    BuildRequest, BuildStatus, LifecycleStatus, SliceMetadata, actions as slice_actions,
    build_request, enforce_report_no_blocking_on_success, enforce_report_outputs_exist,
};

/// The validated result of a completed [`build`], mirroring the native
/// finalize output: the report's slice / target / status plus the
/// finding count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    /// Slice that was built.
    pub slice: String,
    /// Adapter that produced the report (e.g. `omnia@1.0.0`).
    pub target: String,
    /// `success` (a failure report aborts before this is returned).
    pub status: BuildStatus,
    /// Count of (non-blocking) findings on the report.
    pub findings: usize,
}

/// Build one slice through the seam and run the finalize tail.
///
/// Assembles, schema-gates, and persists `build/request.yaml`,
/// journals `target.execution.agent`, then brackets the dispatch +
/// finalize tail with `slice.build.started` / `slice.build.succeeded`
/// / `slice.build.failed`. The tail is the report schema gate,
/// slice-name match, [`enforce_report_no_blocking_on_success`],
/// [`enforce_report_outputs_exist`], failure-status rejection, and the
/// `Refined → Built` transition. The UI-surface coherence judgement
/// lives in the target adapter's own guest.
///
/// `manifest_inputs` is the bound target's declared build-inputs list
/// (empty when the target declares none); `tree` names the snapshot
/// the build applies against — both are caller-resolved by the guest
/// shim.
///
/// # Errors
///
/// - propagates `metadata.yaml` load and request assembly/validation
///   failures (`target-build-input-missing`,
///   `target-build-request-schema`).
/// - `seam-dispatch-failed` when the seam dispatch fails.
/// - `target-build-report-schema` /
///   `target-build-report-slice-mismatch` /
///   `target-build-success-with-blocking-finding` /
///   `target-build-output-missing` / `target-build-failed` and the
///   `lifecycle` gate error from the finalize tail.
pub async fn build(
    seam: &impl TargetSeam, layout: Layout<'_>, now: Timestamp, slice: &str,
    manifest_inputs: &[BuildInputDeclaration], tree: WorkingTree,
) -> Result<BuildOutcome, Error> {
    let slice_dir = layout.slices_dir().join(slice);
    let metadata = SliceMetadata::load(&slice_dir)?;
    let target_name = adapter_ref_from_value(&metadata.target).name;

    let request = assemble_and_write_request(layout, slice, &slice_dir, manifest_inputs)?;

    journal::emit_best_effort(
        layout,
        now,
        EventKind::TargetExecutionAgent {
            slice: slice.into(),
            target: target_name.clone(),
        },
        "slice.build",
    );

    // The `slice.build.*` pair brackets the dispatch *and* the finalize
    // tail: the guest has no prepare/finalize seam for an agent to sit
    // between, so `started` frames the whole operation.
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceBuildStarted {
            slice_name: slice.into(),
        },
        "slice.build",
    );
    match dispatch_and_finalize(seam, layout, now, slice, &slice_dir, &target_name, &request, tree)
        .await
    {
        Ok(outcome) => {
            journal::emit_best_effort(
                layout,
                now,
                EventKind::SliceBuildSucceeded {
                    slice_name: slice.into(),
                },
                "slice.build",
            );
            Ok(outcome)
        }
        Err(err) => {
            journal::emit_best_effort(
                layout,
                now,
                EventKind::SliceBuildFailed {
                    slice_name: slice.into(),
                    reason: err.variant_str().into_owned(),
                },
                "slice.build",
            );
            Err(err)
        }
    }
}

/// Dispatch `seam.build` and run the native finalize tail over the
/// returned report. Wrapped by [`build`] so the `slice.build.*` pair
/// brackets it.
#[expect(clippy::too_many_arguments, reason = "internal seam-dispatch kernel; callers use `build`")]
async fn dispatch_and_finalize(
    seam: &impl TargetSeam, layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path,
    target_name: &str, request: &BuildRequest, tree: WorkingTree,
) -> Result<BuildOutcome, Error> {
    let inputs = read_inputs(request)?;
    let id = target_adapter_id(target_name);
    let report = seam
        .build(id.clone(), slice.to_string(), inputs, tree)
        .await
        .map_err(|err| seam_failure("build", &id, &err))?;

    // Persist + schema-gate the report before anything acts on it, so
    // the on-disk `build/report.yaml` matches what the tail validated
    // (parity with the native finalize reading the agent's file).
    let yaml = trailing_newline_yaml(&report)?;
    validate_build_report_json(&yaml)?;
    bytes_write(&slice_dir.join("build").join("report.yaml"), yaml.as_bytes())?;

    if report.slice != slice {
        return Err(Error::validation_failed(
            "target-build-report-slice-mismatch",
            "the build report's slice matches the slice being finalized",
            format!("report names slice `{}`, but the build ran for `{slice}`", report.slice),
        ));
    }

    enforce_report_no_blocking_on_success(&report)?;
    enforce_report_outputs_exist(&report, layout.project_dir())?;
    if report.status == BuildStatus::Failure {
        return Err(Error::Diag {
            code: "target-build-failed",
            detail: format!(
                "target `{}` reported a failed build for slice `{slice}` ({} finding(s))",
                report.target,
                report.findings.len()
            ),
        });
    }

    slice_actions::transition(slice_dir, LifecycleStatus::Built, now)?;

    Ok(BuildOutcome {
        slice: slice.to_string(),
        target: report.target,
        status: report.status,
        findings: report.findings.len(),
    })
}

/// Assemble the build request from the declared inputs, schema-validate
/// the serialised envelope, and persist it atomically to
/// `.specify/slices/<slice>/build/request.yaml` — the native prepare
/// leg verbatim, minus the shell hooks.
fn assemble_and_write_request(
    layout: Layout<'_>, slice: &str, slice_dir: &Path, manifest_inputs: &[BuildInputDeclaration],
) -> Result<BuildRequest, Error> {
    let request = build_request(slice, manifest_inputs, slice_dir, layout.project_dir())?;
    let yaml = trailing_newline_yaml(&request)?;
    validate_build_request_json(&yaml)?;

    let build_dir = slice_dir.join("build");
    std::fs::create_dir_all(&build_dir).map_err(Error::Io)?;
    bytes_write(&build_dir.join("request.yaml"), yaml.as_bytes())?;
    Ok(request)
}

/// Read the request's resolved artifact bodies into the seam's
/// [`Input`] variants, in request order (proposal, design, tasks,
/// specs, additional).
fn read_inputs(request: &BuildRequest) -> Result<Vec<Input>, Error> {
    let root = &request.inputs.root;
    let artifacts = &request.inputs.artifacts;
    let mut inputs = vec![
        Input::Proposal(read_artifact(root, &artifacts.proposal)?),
        Input::Design(read_artifact(root, &artifacts.design)?),
        Input::Tasks(read_artifact(root, &artifacts.tasks)?),
    ];
    for spec in &artifacts.specs {
        inputs.push(Input::Spec(read_artifact(root, spec)?));
    }
    for additional in &artifacts.additional {
        inputs.push(Input::Other(read_artifact(root, additional)?));
    }
    Ok(inputs)
}

/// Read one slice-tree artifact body.
fn read_artifact(root: &Path, relative: &str) -> Result<String, Error> {
    let path = root.join(relative);
    std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
        op: "read",
        path,
        source,
    })
}

/// Serialise to a trailing-newlined YAML document.
fn trailing_newline_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut content = serde_saphyr::to_string(value)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}
