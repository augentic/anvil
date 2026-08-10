//! Target-axis build orchestrator.

use std::path::Path;

use artifacts::atomic::bytes_write;
use error::Error;
use jiff::Timestamp;
use project::adapter::{AdapterSelector, TargetAdapter};
use project::build_record::BuildRecord;
use project::config::{Layout, ProjectConfig};
use project::journal::{self, EventKind};
use project::name::SliceName;
use project::plan::{Plan, dir_cid};
use project::seam::{BuildContext, Input, Payload, Target, Workspaces};
use project::wave::{EpochRef, Wave};

use super::{seam_failure, target_id};
use crate::{
    Base, BuildRequest, BuildStatus, LifecycleStatus, SliceMetadata, actions as slice_actions,
    build_request,
};

/// The validated result of a completed [`build`], mirroring the native
/// finalize output: the report's slice / target / status plus the
/// finding count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    /// Slice that was built.
    pub slice: String,
    /// Target adapter identifier from the report.
    pub target: String,
    /// Report status.
    pub status: BuildStatus,
    /// Finding count on the report.
    pub findings: usize,
}

/// Build one slice through the seam and run the finalize tail.
///
/// The build runs inside a disposable private workspace prepared from
/// the refine-time target-base pin in `base.yaml` — never an ambient
/// freeze; durable code state is only the captured snapshots.
/// `adapter` is the caller-resolved bound target, and its name must
/// match the slice's recorded `metadata.yaml` target so the declared
/// inputs and the seam dispatch can never resolve differently.
///
/// # Errors
///
/// Refuses with `target-build-adapter-mismatch` when the slice's recorded target
///   names a different adapter than `adapter`.
/// Workspace, dispatch, and finalize failures retain their seam,
/// report, output, or lifecycle diagnostics.
#[tracing::instrument(name = "slice.build", skip_all, fields(slice = %slice, target = %adapter.name))]
pub async fn build(
    seam: &(impl Target + Workspaces), layout: Layout<'_>, now: Timestamp, slice: &str,
    adapter: &TargetAdapter,
) -> Result<BuildOutcome, Error> {
    tracing::info!("build started");
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
    let outcome = journal::bracket(
        layout,
        now,
        "slice.build",
        EventKind::SliceBuildStarted {
            slice_name: slice.into(),
        },
        in_workspace(seam, layout, now, slice, &slice_dir, adapter, &request),
        |_| EventKind::SliceBuildSucceeded {
            slice_name: slice.into(),
        },
        |err| EventKind::SliceBuildFailed {
            slice_name: slice.into(),
            reason: err.variant_str().into_owned(),
        },
    )
    .await?;
    tracing::info!(status = ?outcome.status, "build completed");
    Ok(outcome)
}

/// Bracket [`finalize`] with the workspace lifecycle: read the
/// recorded target-base pin, open a one-member wave, prepare a
/// writable private workspace from that pin, run the dispatch +
/// finalize tail against it, and discard the workspace on every exit
/// (best-effort — captured snapshots survive by digest and a leaked
/// directory is GC territory, never a build failure).
async fn in_workspace(
    seam: &(impl Target + Workspaces), layout: Layout<'_>, now: Timestamp, slice: &str,
    slice_dir: &Path, adapter: &TargetAdapter, request: &BuildRequest,
) -> Result<BuildOutcome, Error> {
    let pins = Base::load(slice_dir).map_err(|err| Error::Diag {
        code: "slice-base-missing",
        detail: format!(
            "slice `{slice}` has no readable base.yaml target-base pin; re-run \
             `emery plan execute` so the refine phase records it before building ({err})"
        ),
    })?;
    let base = pins.target_base;
    let wave_digest = open_wave(layout, now, slice, slice_dir, &base)?;
    let workspace =
        seam.prepare(base, true).await.map_err(|err| workspace_failure("prepare", slice, &err))?;
    let outcome =
        finalize(seam, layout, now, slice, slice_dir, adapter, request, &workspace, wave_digest)
            .await;
    if let Err(err) = seam.discard(workspace.id.clone()).await {
        tracing::warn!(workspace = %workspace.id, "workspace discard failed: {err}");
    }
    outcome
}

/// Open the one-member target wave for this build (RFC-86 D9).
fn open_wave(
    layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path,
    base: &project::snapshot::SnapshotId,
) -> Result<project::snapshot::SnapshotId, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let plan = Plan::load(&layout.plan_path())?;
    let entry =
        plan.entries.iter().find(|e| e.name.as_str() == slice).ok_or_else(|| Error::Diag {
            code: "target-wave-entry-missing",
            detail: format!(
                "slice `{slice}` has no plan.yaml entry; cannot open a target wave without \
                 depends-on / membership"
            ),
        })?;
    let specs_dir = slice_dir.join("specs");
    let wave = Wave::one_member(
        config.name,
        base.clone(),
        SliceName::from(slice),
        dir_cid(&specs_dir)?,
        entry.depends_on.clone(),
        covering_epoch(layout),
    );
    Ok(wave.open(layout, now)?.digest)
}

/// Newest `plan.execute.started` in the fact union, else an unbound
/// `{ writer, sequence: 0 }` ref for breakout builds without an epoch.
fn covering_epoch(layout: Layout<'_>) -> EpochRef {
    let Ok(events) = journal::read_union(layout) else {
        return EpochRef {
            writer: journal::writer_id(),
            sequence: 0,
        };
    };
    events
        .iter()
        .rev()
        .find_map(|event| match event.kind {
            EventKind::PlanExecuteStarted { .. } => Some(EpochRef {
                writer: event.writer.clone(),
                sequence: event.sequence,
            }),
            _ => None,
        })
        .unwrap_or_else(|| EpochRef {
            writer: journal::writer_id(),
            sequence: 0,
        })
}

/// Map a workspace-capability failure onto the build's diagnostic
/// contract.
fn workspace_failure(operation: &'static str, slice: &str, err: &project::seam::Error) -> Error {
    Error::Diag {
        code: "target-build-workspace-failed",
        detail: format!("workspace `{operation}` failed for slice `{slice}`: {err}"),
    }
}

/// Dispatch `seam.build` and run the native finalize tail over the
/// returned report. Wrapped by [`build`] so the `slice.build.*` pair
/// brackets it.
#[expect(clippy::too_many_arguments, reason = "internal seam-dispatch kernel; callers use `build`")]
async fn finalize(
    seam: &(impl Target + Workspaces), layout: Layout<'_>, now: Timestamp, slice: &str,
    slice_dir: &Path, adapter: &TargetAdapter, request: &BuildRequest,
    workspace: &project::seam::Workspace, wave: project::snapshot::SnapshotId,
) -> Result<BuildOutcome, Error> {
    let inputs = read_inputs(request)?;
    let context = build_context(layout, slice)?;
    let id = target_id(adapter);
    let report = seam
        .build(id.clone(), slice.to_string(), inputs, context, workspace.clone())
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
    // A deferred requirement is out of build scope (RFC-86a D4); a
    // report claiming it is a contract violation.
    report.enforce_deferred_not_covered(&request.deferred)?;
    // Declared outputs live in the private workspace until capture.
    report.enforce_outputs_exist(Path::new(&workspace.root))?;
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

    // Capture the result tree and persist the fact-substrate build
    // record (RFC-86 D27) — `build/patch.yaml` is no longer authority.
    let patch = seam
        .capture(workspace.id.clone())
        .await
        .map_err(|err| workspace_failure("capture", slice, &err))?;
    let consumed = request.deferred.iter().map(|req| req.requirement_digest.clone()).collect();
    BuildRecord::from_capture(patch, wave, report.clone(), consumed).write(slice_dir)?;

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
/// `.emery/slices/<slice>/build/request.yaml` — the native prepare
/// leg verbatim, minus the shell hooks.
fn write_request(
    layout: Layout<'_>, slice: &str, slice_dir: &Path,
    manifest_inputs: &[project::adapter::BuildInputDeclaration],
) -> Result<BuildRequest, Error> {
    // The build's exclusion set (RFC-86a D4): every deferred gap row
    // on the slice, projected from the dispositions at request time.
    let deferred = crate::build::deferred::live_deferred(layout, slice)?;
    let request = build_request(slice, manifest_inputs, slice_dir, layout.project_dir(), deferred)?;
    let yaml = project::fs::yaml(&request)?;

    let build_dir = slice_dir.join("build");
    std::fs::create_dir_all(&build_dir).map_err(Error::Io)?;
    bytes_write(&build_dir.join("request.yaml"), yaml.as_bytes())?;
    Ok(request)
}

/// Resolve the request's artifacts into path-form seam [`Input`]s,
/// in request order (proposal, design, tasks, specs, additional).
///
/// Each payload is [`Payload::Path`] with the artifact's
/// project-relative, '/'-separated path — change-tree artifacts stay
/// outside the private workspace, so the adapter reads them through
/// its `"."` preopen and renders them against the workspace's
/// agent-visible artifact root for its agent. Paths are never
/// host-absolute.
fn read_inputs(request: &BuildRequest) -> Result<Vec<Input>, Error> {
    let root = &request.inputs.root;
    let project_dir = &request.project_dir;
    let artifacts = &request.inputs.artifacts;
    let resolve = |relative: &str| resolve_artifact(root, project_dir, relative, true);
    let mut inputs = vec![
        Input::Proposal(resolve(&artifacts.proposal)?),
        Input::Design(resolve(&artifacts.design)?),
        Input::Tasks(resolve(&artifacts.tasks)?),
    ];
    for spec in &artifacts.specs {
        inputs.push(Input::Spec(resolve(spec)?));
    }
    for additional in &artifacts.additional {
        // Adapter-declared inputs may be directories (e.g. the contracts
        // staged-delta dir), so a build retry over an existing delta
        // assembles instead of tripping the file gate.
        inputs.push(Input::Other(resolve_artifact(root, project_dir, additional, false)?));
    }
    Ok(inputs)
}

/// Resolve one request artifact to its project-relative path payload,
/// verifying it exists (`file_only` additionally requires a regular
/// file) so a broken slice tree fails here rather than inside the
/// adapter's judgment leg.
fn resolve_artifact(
    root: &Path, project_dir: &Path, relative: &str, file_only: bool,
) -> Result<Payload, Error> {
    let absolute = root.join(relative);
    let present = if file_only { absolute.is_file() } else { absolute.exists() };
    if !present {
        return Err(Error::validation_failed(
            "target-build-input-missing",
            "every request artifact resolves to a file in the slice tree",
            format!("build input `{}` does not exist", absolute.display()),
        ));
    }
    let project_relative = absolute.strip_prefix(project_dir).map_err(|_prefix| Error::Diag {
        code: "target-build-input-outside-project",
        detail: format!(
            "build input `{}` resolves outside the project directory `{}`",
            absolute.display(),
            project_dir.display()
        ),
    })?;
    let path = project_relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(Payload::Path(path))
}

/// Resolve the slice's bound source-adapter names from its plan entry
/// into the seam [`BuildContext`].
///
/// Each entry binding's source key maps through the plan-level
/// `sources` table to its adapter name; names dedupe in first-bound
/// order. A missing `plan.yaml`, an absent entry, or an unresolvable
/// key yields an empty / partial list rather than a refusal — the
/// context is advisory (targets use it to skip source-conditional
/// legs), not a gate.
fn build_context(layout: Layout<'_>, slice: &str) -> Result<BuildContext, Error> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Ok(BuildContext::default());
    }
    let plan = Plan::load(&plan_path)?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == slice) else {
        return Ok(BuildContext::default());
    };
    let mut sources = Vec::new();
    for binding in &entry.sources {
        if let Some(bound) = plan.sources.get(&binding.source)
            && !sources.contains(&bound.adapter)
        {
            sources.push(bound.adapter.clone());
        }
    }
    Ok(BuildContext { sources })
}
