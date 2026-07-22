//! Target-axis build orchestrator.

use std::path::Path;

use artifacts::atomic::bytes_write;
use error::Error;
use jiff::Timestamp;
use project::adapter::{AdapterSelector, TargetAdapter};
use project::config::Layout;
use project::journal::{self, EventKind};
use project::seam::{Input, Target, WorkingTree};

use super::{seam_failure, target_id};
use crate::{
    BuildRequest, BuildStatus, LifecycleStatus, SliceMetadata, actions as slice_actions,
    build_request,
};

/// The validated result of a completed [`build`], mirroring the native
/// finalize output: the report's slice / target / status plus the
/// finding count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub slice: String,
    pub target: String,
    pub status: BuildStatus,
    pub findings: usize,
}

/// Build one slice through the seam and run the finalize tail.
///
/// Assembles and persists `build/request.yaml`,
/// journals `target.execution.agent`, then brackets the dispatch +
/// finalize tail with `slice.build.started` / `slice.build.succeeded`
/// / `slice.build.failed`. The tail is the
/// slice-name match, [`crate::BuildReport::enforce_no_blocking`],
/// [`crate::BuildReport::enforce_outputs_exist`], failure-status rejection, and the
/// `Refined → Built` transition. The UI-surface coherence judgement
/// lives in the target adapter's own guest.
///
/// `adapter` is the caller-resolved bound target adapter — its
/// declared `inputs[]` assemble the request, and its name must match
/// the slice's recorded `metadata.yaml` target so the declared inputs
/// and the seam dispatch can never resolve from different adapters.
/// `tree` names the snapshot the build applies against.
///
/// # Errors
///
/// Refuses with `target-build-adapter-mismatch` when the slice's recorded target
///   names a different adapter than `adapter`.
/// Dispatch and finalize failures retain their seam, report, output, or
/// lifecycle diagnostics.
pub async fn build(
    seam: &impl Target, layout: Layout<'_>, now: Timestamp, slice: &str, adapter: &TargetAdapter,
    tree: WorkingTree,
) -> Result<BuildOutcome, Error> {
    let slice_dir = layout.slice_dir(slice);
    let metadata = SliceMetadata::load(&slice_dir)?;
    let target_name = AdapterSelector::recorded_name(&metadata.target);
    if target_name != adapter.name {
        return Err(Error::validation_failed(
            "target-build-adapter-mismatch",
            "the slice's recorded target names the resolved build adapter",
            format!(
                "slice `{slice}` records target `{}` but the build resolved adapter `{}`; align \
                 the project's declared adapter with the slice (or re-create the slice) before \
                 building",
                metadata.target, adapter.name
            ),
        ));
    }

    let request = write_request(layout, slice, &slice_dir, &adapter.inputs)?;

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
    journal::bracket(
        layout,
        now,
        "slice.build",
        EventKind::SliceBuildStarted {
            slice_name: slice.into(),
        },
        finalize(seam, layout, now, slice, &slice_dir, adapter, &request, tree),
        |_| EventKind::SliceBuildSucceeded {
            slice_name: slice.into(),
        },
        |err| EventKind::SliceBuildFailed {
            slice_name: slice.into(),
            reason: err.variant_str().into_owned(),
        },
    )
    .await
}

/// Dispatch `seam.build` and run the native finalize tail over the
/// returned report. Wrapped by [`build`] so the `slice.build.*` pair
/// brackets it.
#[expect(clippy::too_many_arguments, reason = "internal seam-dispatch kernel; callers use `build`")]
async fn finalize(
    seam: &impl Target, layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path,
    adapter: &TargetAdapter, request: &BuildRequest, tree: WorkingTree,
) -> Result<BuildOutcome, Error> {
    let inputs = read_inputs(request)?;
    let id = target_id(adapter);
    let report = seam
        .build(id.clone(), slice.to_string(), inputs, tree)
        .await
        .map_err(|err| seam_failure("build", &id, &err))?;

    // Persist the typed report before anything acts on it, so the
    // on-disk `build/report.yaml` matches what the tail validated
    // (parity with the native finalize reading the agent's file).
    let yaml = project::fs::yaml(&report)?;
    bytes_write(&slice_dir.join("build").join("report.yaml"), yaml.as_bytes())?;

    if report.slice != slice {
        return Err(Error::validation_failed(
            "target-build-report-slice-mismatch",
            "the build report's slice matches the slice being finalized",
            format!("report names slice `{}`, but the build ran for `{slice}`", report.slice),
        ));
    }

    report.enforce_no_blocking()?;
    report.enforce_outputs_exist(layout.project_dir())?;
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

/// Assemble the build request from the declared inputs and persist it
/// atomically to
/// `.specify/slices/<slice>/build/request.yaml` — the native prepare
/// leg verbatim, minus the shell hooks.
fn write_request(
    layout: Layout<'_>, slice: &str, slice_dir: &Path,
    manifest_inputs: &[project::adapter::BuildInputDeclaration],
) -> Result<BuildRequest, Error> {
    let request = build_request(slice, manifest_inputs, slice_dir, layout.project_dir())?;
    let yaml = project::fs::yaml(&request)?;

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

fn read_artifact(root: &Path, relative: &str) -> Result<String, Error> {
    project::fs::read_text(&root.join(relative))
}
