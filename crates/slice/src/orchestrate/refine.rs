//! The refine-phase orchestrator behind `/spec:refine`: slice create
//! (re-entry safe), per-binding extract fan-out, the synthesis
//! judgment leg, the persist tail, the validate gate sweep, and the
//! `refined` transition. A validate failure leaves the slice
//! `refining` and fires no `slice.synthesize.failed`.

use std::path::{Path, PathBuf};

use artifacts::spec::provenance::RequirementTag;
use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::{AdapterSelector, Resolver};
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::journal::{self, EventKind};
use project::plan::{Entry, Plan, Status, resolve_topology};
use project::registry::topology::{Decision, Surface};
use project::seam::{Source, Target};

use super::synthesize::SynthesizeRequest;
use crate::judgment::synthesize::Kernel;
use crate::merge::{MergeStrategy, artifact_classes};
use crate::validate::{Validation, append_synthesis_journal};
use crate::{
    BaselineIndex, CreateIfExists, DomainDetail, LifecycleStatus, ProjectionHeader,
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
    /// Synthesis-tag counts from the validate sweep's spec scan —
    /// review signals, never a park.
    pub tags: TagCounts,
}

/// Per-tag requirement counts gathered by the validate sweep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TagCounts {
    /// `[unknown]` requirements.
    pub unknown: usize,
    /// `[conflict]` requirements.
    pub conflict: usize,
    /// `[divergence]` requirements.
    pub divergence: usize,
}

impl TagCounts {
    /// Tally `(requirement-id, tag)` pairs from the spec scan.
    fn tally(tags: &[(String, RequirementTag)]) -> Self {
        let mut counts = Self::default();
        for (_, tag) in tags {
            match tag {
                RequirementTag::Unknown => counts.unknown += 1,
                RequirementTag::Conflict => counts.conflict += 1,
                RequirementTag::Divergence => counts.divergence += 1,
            }
        }
        counts
    }
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
pub async fn refine<P: Model, S: Source, T: Target, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, slice: &str,
    target_value: &str,
) -> Result<RefineOutcome, Error> {
    let layout = Layout::new(paths.project_root());
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
        super::extract(caps.sources, layout, now, &source, &lead, slice).await?;
        extracted.push((source, lead));
    }

    // Assemble the kernel context the judgment leg projects against.
    let source_inputs = read_source_inputs(&slice_dir, &entry)?;
    let (authority, evidence_claims) = read_evidence_index(&slice_dir, &entry)?;
    let overrides = entry.authority_override.by_kind.clone();
    let baseline_specs_dir = baseline_specs_dir(layout, &slice_dir);
    let baseline_index = BaselineIndex::build(&baseline_specs_dir)?;
    let (baseline, baseline_decisions) = baseline_identity(caps.resolver, paths, &entry)?;
    let baseline_detail: Vec<DomainDetail> = (&baseline_index).into();
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
        baseline_decisions: &baseline_decisions,
    };

    // Synthesis is model-dispatched — record the handoff, then bracket
    // the judgment-plus-persist leg with the native started /
    // completed / failed pair. Emits are best-effort: a journal hiccup
    // never shadows the synthesis outcome (the native handler's
    // posture).
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceSynthesizeAgent {
            slice_name: slice.into(),
        },
        SYNTHESIZE_SCOPE,
    );
    let artifacts = journal::bracket(
        layout,
        now,
        SYNTHESIZE_SCOPE,
        EventKind::SliceSynthesizeStarted {
            slice_name: slice.into(),
        },
        synthesize_and_persist(
            caps.model,
            caps.targets,
            &request,
            &kernel,
            &slice_dir,
            &baseline_index,
        ),
        |artifacts: &Vec<String>| EventKind::SliceSynthesizeCompleted {
            slice_name: slice.into(),
            artifacts: artifacts.clone(),
        },
        |err| EventKind::SliceSynthesizeFailed {
            slice_name: slice.into(),
            reason: synthesize_failure_reason(err),
        },
    )
    .await?;

    let tags = validate(layout, now, slice)?;

    slice_actions::transition(&slice_dir, LifecycleStatus::Refined, now)?;

    Ok(RefineOutcome {
        slice: slice.to_string(),
        artifacts,
        extracted,
        tags,
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
pub async fn refine_breakout<P: Model, S: Source, T: Target, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, slice: &str,
) -> Result<RefineOutcome, Error> {
    let layout = Layout::new(paths.project_root());
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
    let target = breakout_target(caps.resolver, paths, &entry, slice)?;
    refine(caps, paths, now, slice, &target).await
}

/// Resolve the breakout's target value: the slice's recorded
/// `metadata.yaml` target when the slice directory already exists
/// (resumed policy), else the bound project's topology (fresh policy).
fn breakout_target(
    resolver: &impl Resolver, paths: &ExecutionPaths, entry: &Entry, slice: &str,
) -> Result<String, Error> {
    project::target_policy::resumed(Layout::new(paths.project_root()), slice)
        .or_else(|_| project::target_policy::fresh(resolver, paths, entry, slice, "refining"))
}

/// The judgment leg plus the native persist tail — one fallible unit
/// so the `slice.synthesize.*` pair brackets both.
async fn synthesize_and_persist<P: Model, T: Target>(
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
/// decision and error codes match the native verb). Returns the tag
/// counts the refine body surfaces.
fn validate(layout: Layout<'_>, now: Timestamp, slice: &str) -> Result<TagCounts, Error> {
    match crate::validate::run(layout, slice)? {
        Validation::Gate { code, findings } => Err(Error::validation_failed(
            code,
            "slice must satisfy structural invariants",
            format!("{} blocking finding(s)", findings.len()),
        )),
        Validation::Adapter {
            findings,
            synthesis_tags,
        } => {
            if has_blocking(&findings) {
                let rules: Vec<&str> = findings
                    .iter()
                    .filter(|finding| diagnostics::is_blocking(finding))
                    .map(|finding| finding.rule_id.as_deref().unwrap_or("unnamed-rule"))
                    .collect();
                return Err(Error::validation_failed(
                    "slice-validation-failed",
                    "slice must satisfy adapter validation",
                    format!("slice `{slice}` failed validation: {}", rules.join(", ")),
                ));
            }
            let counts = TagCounts::tally(&synthesis_tags);
            append_synthesis_journal(layout, now, slice, synthesis_tags)?;
            Ok(counts)
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

/// The `ThreeWayMerge` baseline `specs/` directory — the same path
/// the synthesis persist tail and merge resolve.
fn baseline_specs_dir(layout: Layout<'_>, slice_dir: &Path) -> PathBuf {
    let classes = artifact_classes(layout.project_dir(), slice_dir);
    classes
        .iter()
        .find(|class| matches!(class.strategy, MergeStrategy::ThreeWayMerge))
        .map_or_else(|| layout.specs_dir(), |class| class.baseline_dir.clone())
}

/// The slice's bound-project baseline identity for the synthesis
/// inputs envelope: its `surface[]` plus the accepted baseline
/// Decision Record projection (`decisions[]`). Baseline is advisory
/// context, so any topology resolution miss degrades to empty vectors
/// (the native handler's posture).
fn baseline_identity(
    resolver: &impl Resolver, paths: &ExecutionPaths, entry: &Entry,
) -> Result<(Vec<Surface>, Vec<Decision>), Error> {
    let config = ProjectConfig::load(Layout::new(paths.project_root()).project_dir())?;
    let topology = resolve_topology(resolver, &config, paths)?;
    let bound = match entry.project.as_deref() {
        Some(name) => topology.iter().find(|p| p.name == name),
        None if topology.len() == 1 => topology.first(),
        None => None,
    };
    Ok(bound.map(|p| (p.surface.clone(), p.decisions.clone())).unwrap_or_default())
}

/// Bare adapter name from a recorded target value (`omnia@1.0.0` →
/// `omnia`) — the seam routes by the plan-bound name.
fn target_name(target_value: &str) -> String {
    AdapterSelector::recorded_name(target_value)
}

/// Warning scope for the best-effort `slice.synthesize.*` brackets.
const SYNTHESIZE_SCOPE: &str = "slice.synthesize";
