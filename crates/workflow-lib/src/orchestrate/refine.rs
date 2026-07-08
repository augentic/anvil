//! The refine-phase orchestrator: the guest collapse of the
//! `/spec:refine` critical path.
//!
//! One call composes what the skill drives as five CLI invocations:
//! `slice create --if-exists continue` (re-entry safe — a slice parked
//! at `refining` resumes), the per-binding `source extract` fan-out
//! (via [`super::extract`]), the synthesis judgment leg with seam
//! guidance (via [`super::synthesize`]), the persist tail
//! ([`persist_synthesized`]), `slice validate`'s gate sweep + adapter
//! rules, and the `refined` transition.
//!
//! Journal cadence composes the native verbs': extract events from
//! [`super::extract`], then `slice.synthesize.agent` (the handoff is a
//! model dispatch), `slice.synthesize.started` /
//! `slice.synthesize.completed` / `slice.synthesize.failed` around the
//! judgment-plus-persist leg, the validate sweep's synthesis-tag
//! events, and `slice.transition.refined` from the transition. A
//! validate failure carries no `slice.synthesize.failed` — matching
//! native, where validate is a separate verb; the slice stays
//! `refining` either way.

use std::path::{Path, PathBuf};

use diagnostics::blocking_present;
use error::Error;
use guest_model::Model;
use jiff::Timestamp;

use super::synthesize::SynthesizeRequest;
use crate::change::{Entry, Plan, Status, resolve_target, resolve_topology};
use crate::config::{Layout, ProjectConfig};
use crate::journal::{self, EventKind};
use crate::judgment::synthesize::Kernel;
use crate::merge::{MergeStrategy, artifact_classes};
use crate::registry::topology::Surface;
use crate::seam::{SourceSeam, TargetSeam};
use crate::slice::validate::{PreAdapter, append_synthesis_journal, pre_adapter_gates};
use crate::slice::{
    BaselineDomainDetail, BaselineIndex, CreateIfExists, LifecycleStatus, ProjectionHeader,
    actions as slice_actions, persist_synthesized, read_evidence_index, read_source_inputs,
    synthesize_failure_reason,
};

/// The result of a completed [`refine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefineOutcome {
    /// Slice that was refined.
    pub slice: String,
    /// Slice-relative artifact paths persisted by the synthesis tail.
    pub artifacts: Vec<String>,
    /// `(source, lead)` pairs extracted, in binding order.
    pub extracted: Vec<(String, String)>,
}

/// Refine one plan entry's slice to `refined`.
///
/// `target_value` is the resolved target the slice's `metadata.yaml`
/// records (e.g. `omnia@1.0.0`) — caller-resolved, mirroring how the
/// skill takes it from the `plan next` response.
///
/// # Errors
///
/// - `slice-synthesize-plan-missing` / `slice-synthesize-entry-missing`
///   when the plan or the slice's entry is absent.
/// - slice-create failures (`invalid-name`,
///   `slice-dir-missing-metadata`).
/// - extract failures from [`super::extract`].
/// - the judgment leg's model / schema / kernel failures and persist
///   failures (journalled as `slice.synthesize.failed`).
/// - `slice-provenance-invalid` / `slice-pre-adapter-gate` /
///   `slice-validation-failed` from the validate sweep.
/// - the `lifecycle` gate error from the `refined` transition.
pub async fn refine<P: Model, S: SourceSeam, T: TargetSeam>(
    model: &P, sources: &S, targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str,
    target_value: &str,
) -> Result<RefineOutcome, Error> {
    let entry = load_entry(layout, slice)?;
    let parent_dir = layout.slices_dir();
    std::fs::create_dir_all(&parent_dir).map_err(Error::Io)?;
    let created =
        slice_actions::create(&parent_dir, slice, target_value, CreateIfExists::Continue, now)?;
    let slice_dir = created.dir;

    // Extract fan-out, serially in binding declaration order (the
    // skill's no-parallelism rule).
    let mut extracted = Vec::with_capacity(entry.sources.len());
    for binding in &entry.sources {
        let source = binding.source().to_string();
        let lead = binding.lead(slice).to_string();
        super::extract(sources, layout, now, &source, &lead, slice).await?;
        extracted.push((source, lead));
    }

    // Assemble the kernel context the judgment leg projects against.
    let source_inputs = read_source_inputs(&slice_dir, &entry)?;
    let (authority, evidence_claims) = read_evidence_index(&slice_dir, &entry)?;
    let overrides = entry.authority_override.by_kind.clone();
    let baseline_specs_dir = resolve_baseline_specs_dir(layout, &slice_dir);
    let baseline_index = BaselineIndex::build(&baseline_specs_dir)?;
    let baseline = baseline_surface(layout, &entry)?;
    let baseline_detail: Vec<BaselineDomainDetail> = (&baseline_index).into();
    let header = ProjectionHeader {
        version: 1,
        slice: slice.to_string(),
        project: entry.project.clone(),
    };
    let kernel = Kernel {
        header,
        authority: &authority,
        overrides: &overrides,
        evidence_claims: &evidence_claims,
        baseline_index: &baseline_index,
    };
    let request = SynthesizeRequest {
        slice,
        target: &target_name(target_value),
        sources: &source_inputs,
        baseline: &baseline,
        baseline_detail: &baseline_detail,
    };

    // Synthesis is model-dispatched — record the handoff, then bracket
    // the judgment-plus-persist leg with the native started /
    // completed / failed pair.
    emit(
        layout,
        now,
        EventKind::SliceSynthesizeAgent {
            slice_name: slice.into(),
        },
    );
    emit(
        layout,
        now,
        EventKind::SliceSynthesizeStarted {
            slice_name: slice.into(),
        },
    );
    let artifacts = match synthesize_and_persist(
        model,
        targets,
        &request,
        &kernel,
        &slice_dir,
        &baseline_index,
    )
    .await
    {
        Ok(artifacts) => {
            emit(
                layout,
                now,
                EventKind::SliceSynthesizeCompleted {
                    slice_name: slice.into(),
                    artifacts: artifacts.clone(),
                },
            );
            artifacts
        }
        Err(err) => {
            emit(
                layout,
                now,
                EventKind::SliceSynthesizeFailed {
                    slice_name: slice.into(),
                    reason: synthesize_failure_reason(&err),
                },
            );
            return Err(err);
        }
    };

    validate(layout, now, slice, &slice_dir)?;

    slice_actions::transition(&slice_dir, LifecycleStatus::Refined, now)?;

    Ok(RefineOutcome {
        slice: slice.to_string(),
        artifacts,
        extracted,
    })
}

/// Refine one named plan entry outside the execute loop — the guest
/// breakout of `/spec:refine`.
///
/// Claim semantics mirror the standalone `slice build <name>` posture:
/// the verb acts on the named slice directly against a `pending` or
/// `in-progress` plan entry, never advancing per-entry status (`plan
/// next` stays the only `in-progress` writer), and refuses a `done`
/// entry. The target is caller-free: it resolves from the slice's own
/// `metadata.yaml` when the slice already exists (a resumed
/// `refining` breakout), else from the bound project's topology — the
/// same resolution `plan next` hands the execute loop.
///
/// # Errors
///
/// - `slice-refine-entry-done` when the entry has already merged.
/// - `slice-create-target-missing` when neither the slice metadata nor
///   the topology resolves a target.
/// - everything [`refine`] surfaces.
pub async fn refine_breakout<P: Model, S: SourceSeam, T: TargetSeam>(
    model: &P, sources: &S, targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str,
) -> Result<RefineOutcome, Error> {
    let entry = load_entry(layout, slice)?;
    if entry.status == Status::Done {
        return Err(Error::validation_failed(
            "slice-refine-entry-done",
            "the plan entry is still open",
            format!(
                "plan entry `{slice}` is already `done`; walk it back with `specify plan \
                 transition {slice} --undo` before re-refining"
            ),
        ));
    }
    let target = breakout_target(layout, &entry, slice)?;
    refine(model, sources, targets, layout, now, slice, &target).await
}

/// Resolve the breakout's target value: the slice's recorded
/// `metadata.yaml` target when the slice directory already exists,
/// else the bound project's topology.
fn breakout_target(layout: Layout<'_>, entry: &Entry, slice: &str) -> Result<String, Error> {
    if let Ok(metadata) = crate::slice::SliceMetadata::load(&layout.slices_dir().join(slice)) {
        return Ok(metadata.target);
    }
    let config = ProjectConfig::load(layout.project_dir())?;
    let topology = resolve_topology(&config, layout.project_dir())?;
    resolve_target(entry, &topology).map(|target| target.to_string()).map_err(|err| Error::Diag {
        code: "slice-create-target-missing",
        detail: format!(
            "no target resolved for slice `{slice}`: {err}; declare the project adapter (or fix \
             the bound project's topology) before refining"
        ),
    })
}

/// The judgment leg plus the native persist tail — one fallible unit
/// so the `slice.synthesize.*` pair brackets both.
async fn synthesize_and_persist<P: Model, T: TargetSeam>(
    model: &P, targets: &T, request: &SynthesizeRequest<'_>, kernel: &Kernel<'_>, slice_dir: &Path,
    baseline_index: &BaselineIndex,
) -> Result<Vec<String>, Error> {
    let synthesized = super::synthesize(model, targets, request, kernel).await?;
    persist_synthesized(
        slice_dir,
        synthesized.response.artifacts,
        &synthesized.projected,
        baseline_index,
    )
}

/// The `slice validate` sweep: pre-adapter gates, adapter rules, and
/// the synthesis-tag journal emission — minus the report rendering
/// (the orchestrator has no stdout report surface; the blocking
/// decision and error codes match the native verb).
fn validate(
    layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path,
) -> Result<(), Error> {
    match pre_adapter_gates(layout, slice)? {
        PreAdapter::Gate { code, findings } => Err(Error::validation_failed(
            code,
            "slice must satisfy structural invariants",
            format!("{} blocking finding(s)", findings.len()),
        )),
        PreAdapter::Proceed {
            synthesis_tags,
            mut advisories,
        } => {
            let mut findings = artifacts::validate::validate_slice(slice_dir)?;
            findings.append(&mut advisories);
            if blocking_present(&findings) {
                let rules: Vec<&str> = findings
                    .iter()
                    .filter(|finding| diagnostics::blocking(finding))
                    .map(|finding| finding.rule_id.as_deref().unwrap_or("unnamed-rule"))
                    .collect();
                return Err(Error::validation_failed(
                    "slice-validation-failed",
                    "slice must satisfy adapter validation",
                    format!("slice `{slice}` failed validation: {}", rules.join(", ")),
                ));
            }
            append_synthesis_journal(layout, now, slice, synthesis_tags)
        }
    }
}

/// Load the named slice's plan entry — the binding that carries the
/// slice's bound `sources[]`, `project`, and per-slice
/// `authority-override`. Mirrors the native synthesize handler's
/// errors.
fn load_entry(layout: Layout<'_>, slice: &str) -> Result<Entry, Error> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Err(Error::validation_failed(
            "slice-synthesize-plan-missing",
            "synthesize reads the slice's bound sources from plan.yaml",
            format!(
                "no plan.yaml at {}; synthesize binds a slice's sources through its plan entry",
                plan_path.display()
            ),
        ));
    }
    let plan = Plan::load(&plan_path)?;
    plan.entries.into_iter().find(|e| e.name == slice).ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-entry-missing",
            "the slice has a matching plan entry",
            format!("plan.yaml has no entry named `{slice}`"),
        )
    })
}

/// The `ThreeWayMerge` baseline `specs/` directory — the same path the
/// native synthesize handler, merge, and `slice touched-specs --scan`
/// resolve.
fn resolve_baseline_specs_dir(layout: Layout<'_>, slice_dir: &Path) -> PathBuf {
    let classes = artifact_classes(layout.project_dir(), slice_dir);
    classes
        .iter()
        .find(|class| matches!(class.strategy, MergeStrategy::ThreeWayMerge))
        .map_or_else(|| layout.specify_dir().join("specs"), |class| class.baseline_dir.clone())
}

/// The slice's bound-project baseline surface for the
/// synthesis inputs envelope. Baseline is advisory context, so any
/// topology resolution miss degrades to an empty surface (the native
/// handler's posture).
fn baseline_surface(layout: Layout<'_>, entry: &Entry) -> Result<Vec<Surface>, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let topology = resolve_topology(&config, layout.project_dir())?;
    let bound = match entry.project.as_deref() {
        Some(name) => topology.iter().find(|p| p.name == name),
        None if topology.len() == 1 => topology.first(),
        None => None,
    };
    Ok(bound.map(|p| p.surface.clone()).unwrap_or_default())
}

/// Bare adapter name from a recorded target value (`omnia@1.0.0` →
/// `omnia`) — the seam routes by the plan-bound name.
fn target_name(target_value: &str) -> String {
    crate::init::adapter_ref_from_value(target_value).name
}

/// Best-effort emit of a single `slice.synthesize.*` journal event —
/// the native handler's posture (a journal hiccup never shadows the
/// synthesis outcome).
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) {
    journal::emit_best_effort(layout, now, kind, "slice.synthesize");
}
