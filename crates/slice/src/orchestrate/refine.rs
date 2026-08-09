//! The refine-phase orchestrator, driven by the execute loop.
//!
//! A validate failure leaves the slice `refining` and fires no
//! `slice.synthesize.failed`.

use std::path::{Path, PathBuf};

use artifacts::spec::provenance::RequirementTag;
use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::identity::{Decision, Surface};
use project::journal::{self, EventKind};
use project::plan::{Entry, Plan, resolve_topology};
use project::seam::{Source, Target, Workspaces};

use super::synthesize::SynthesizeRequest;
use crate::judgment::synthesize::Kernel;
use crate::merge::{MergeStrategy, artifact_classes};
use crate::validate::{Validation, append_synthesis_journal};
use crate::{
    Base, BaselineIndex, CreateIfExists, DomainDetail, LifecycleStatus, ProjectionHeader,
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
/// records (e.g. `omnia@1.0.0`) — caller-resolved by the execute
/// loop's advance step.
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
/// - the `slice-lifecycle` gate error from the `refined` transition.
#[tracing::instrument(name = "slice.refine", skip_all, fields(slice = %slice, target = %target_value))]
pub async fn refine<P: Model, S: Source, T: Target + Workspaces, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, slice: &str,
    target_value: &str,
) -> Result<RefineOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    tracing::info!("refine started");
    let (plan, entry) = load_entry(layout, slice)?;
    let parent_dir = layout.slices_dir();
    std::fs::create_dir_all(&parent_dir).map_err(Error::Io)?;
    let created =
        slice_actions::create(&parent_dir, slice, target_value, CreateIfExists::Continue, now)?;
    let slice_dir = created.dir;

    // Pin assembly before extract (RFC-86 D4 / D25 / D27): copy closed
    // plan source cids, baseline-spec digest, and freeze the product
    // tree as the target-base pin build will prepare from.
    let baseline_specs_dir = baseline_specs_dir(layout, &slice_dir);
    let target_base = caps.targets.freeze().await.map_err(|err| Error::Diag {
        code: "slice-base-freeze-failed",
        detail: format!("freezing the product tree for slice `{slice}` base.yaml failed: {err}"),
    })?;
    Base::assemble(&plan, &entry, &baseline_specs_dir, target_base)?.write(&slice_dir)?;

    // Extract fan-out, serially in binding declaration order (the
    // skill's no-parallelism rule).
    let mut extracted = Vec::with_capacity(entry.sources.len());
    for binding in &entry.sources {
        let source = binding.source().to_string();
        let lead = binding.lead(slice).to_string();
        super::extract(caps.sources, caps.resolver, paths, now, &source, &lead, slice).await?;
        extracted.push((source, lead));
    }

    // Assemble the kernel context the judgment leg projects against.
    let source_inputs = read_source_inputs(layout, &entry)?;
    let (authority, evidence_claims) = read_evidence_index(&slice_dir, &entry)?;
    let overrides = entry.authority_override.by_kind.clone();
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
        target: target_value,
        sources: &source_inputs,
        baseline: &baseline,
        baseline_detail: &baseline_detail,
        baseline_decisions: &baseline_decisions,
    };

    // Synthesis is model-dispatched: record the handoff, then bracket
    // the judgment-plus-persist leg. Emits are best-effort — a journal
    // hiccup never shadows the synthesis outcome.
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
    tracing::info!(artifacts = artifacts.len(), "refine completed");

    Ok(RefineOutcome {
        slice: slice.to_string(),
        artifacts,
        extracted,
        tags,
    })
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

/// Load the plan and the named slice's entry — the binding that
/// carries the slice's bound `sources[]`, `project`, and per-slice
/// `authority-override`. Mirrors the native synthesize handler's
/// errors.
fn load_entry(layout: Layout<'_>, slice: &str) -> Result<(Plan, Entry), Error> {
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
    let entry = plan.entries.iter().find(|e| e.name == slice).cloned().ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-entry-missing",
            "the slice has a matching plan entry",
            format!("plan.yaml has no entry named `{slice}`"),
        )
    })?;
    Ok((plan, entry))
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

/// Warning scope for the best-effort `slice.synthesize.*` brackets.
const SYNTHESIZE_SCOPE: &str = "slice.synthesize";
