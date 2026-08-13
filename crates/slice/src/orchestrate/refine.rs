//! The refine-phase orchestrator, driven by the `plan refine` drain.
//!
//! A validate failure leaves the slice `refining` and fires no
//! `slice.synthesize.failed`; a failed refinement writes no manifest.

use std::path::{Path, PathBuf};

use artifacts::leads::Leads;
use artifacts::spec::provenance::RequirementTag;
use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::{BuildInputDeclaration, Resolver};
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::identity::{Decision, Surface};
use project::journal::{self, EventKind};
use project::plan::{Entry, FocusParent, Plan, resolve_topology};
use project::profile::{Profile, Profiles};
use project::seam::{Source, Target, Workspaces};
use project::snapshot::SnapshotId;

use super::synthesize::SynthesizeRequest;
use crate::judgment::synthesize::Kernel;
use crate::merge::{MergeStrategy, artifact_classes};
use crate::refinement::{self, Dependency};
use crate::synthesis::wire::SynthesisKind;
use crate::validate::{Validation, append_synthesis_journal};
use crate::{
    BaselineIndex, CreateIfExists, DependencyContext, DomainDetail, LifecycleStatus,
    ProjectionHeader, actions as slice_actions, persist_synthesized, read_evidence_index,
    read_source_inputs, synthesize_failure_reason,
};

/// The result of a completed [`refine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefineOutcome {
    /// Synthesis promoted the bundle and the slice is `refined`.
    Refined {
        /// Slice that was refined.
        slice: String,
        /// Slice-relative artifact paths persisted by the synthesis tail.
        artifacts: Vec<String>,
        /// `(source, lead)` pairs extracted, in binding order.
        extracted: Vec<(String, String)>,
        /// Synthesis-tag counts from the validate sweep's spec scan —
        /// review signals, never a park.
        tags: TagCounts,
    },
    /// Evidence-informed boundary escalation: no promotion, no
    /// `refined` transition. The caller persists an inert proposal.
    Escalated {
        /// Slice that escalated.
        slice: String,
        /// Closed assessment from the judgment.
        assessment: project::profile::Assessment,
        /// Named terminal pairs.
        affected: Vec<FocusParent>,
        /// Typed rationale.
        rationale: String,
        /// Bound profile identity the assessment was scored against.
        profile: project::plan::ProfileRef,
        /// `(source, lead)` pairs extracted, in binding order.
        extracted: Vec<(String, String)>,
    },
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

/// Refine one plan entry's slice: extract, synthesize, then either
/// promote to `refined` or return a validated boundary escalation.
///
/// `target_value` is the caller-resolved target recorded in
/// `metadata.yaml`; `dependencies` are the ordered predecessor
/// `(slice, refinement-digest)` pairs (RFC-91 D3); `declarations` is
/// the bound target's build-inputs list the manifest bundle mirrors.
/// A failed or escalated refinement writes no manifest.
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
/// - `slice-refinement-*` / `target-build-input-missing` from the
///   manifest assembly, and filesystem failures from its write.
#[tracing::instrument(name = "slice.refine", skip_all, fields(slice = %slice, target = %target_value))]
pub async fn refine<P: Model + Profiles, S: Source + Workspaces, T: Target, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, slice: &str,
    target_value: &str, dependencies: Vec<Dependency>, declarations: &[BuildInputDeclaration],
) -> Result<RefineOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    tracing::info!("refine started");
    let (plan, entry) = load_entry(layout, slice)?;
    let parent_dir = layout.slices_dir();
    std::fs::create_dir_all(&parent_dir).map_err(Error::Io)?;
    let created =
        slice_actions::create(&parent_dir, slice, target_value, CreateIfExists::Continue, now)?;
    let slice_dir = created.dir;
    let baseline_specs_dir = baseline_specs_dir(layout, &slice_dir);

    let extracted = extract_all(&caps, paths, now, slice, &entry).await?;

    // Assemble the kernel context the judgment leg projects against.
    let source_inputs = read_source_inputs(layout, &entry)?;
    let (authority, evidence_claims) = read_evidence_index(&slice_dir, &entry)?;
    let overrides = entry.authority_override.by_kind.clone();
    let baseline_index = BaselineIndex::build(&baseline_specs_dir)?;
    let (baseline, baseline_decisions) = baseline_identity(paths, &entry);
    let baseline_detail: Vec<DomainDetail> = (&baseline_index).into();
    let dependency_context = dependency_context(layout, &dependencies);
    let header = ProjectionHeader {
        version: 1,
        slice: slice.to_string(),
        target: Some(entry.target.clone()),
    };
    let profile = bound_profile(caps.model, &plan, &entry)?;
    let profile_ref = profile.reference()?;
    let terminals: Vec<FocusParent> = entry
        .sources
        .iter()
        .map(|binding| FocusParent {
            source: binding.source().to_string(),
            lead: binding.lead(slice).to_string(),
        })
        .collect();
    let kernel = Kernel {
        header,
        authority: &authority,
        overrides: &overrides,
        evidence_claims: &evidence_claims,
        baseline_index: &baseline_index,
        profile,
        terminals,
    };
    let request = SynthesizeRequest {
        slice,
        target: target_value,
        sources: &source_inputs,
        baseline: &baseline,
        baseline_detail: &baseline_detail,
        baseline_decisions: &baseline_decisions,
        dependencies: &dependency_context,
    };

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceSynthesizeAgent {
            slice_name: slice.into(),
        },
        SYNTHESIZE_SCOPE,
    );
    let (synthesized, guidance_digest) =
        super::synthesize(caps.model, caps.targets, &request, &kernel).await?;
    if synthesized.response.kind == SynthesisKind::BoundaryEscalation {
        tracing::info!("refine escalated");
        return Ok(escalated(slice, synthesized, profile_ref, extracted));
    }

    let (artifacts, guidance_digest) = journal::bracket(
        layout,
        now,
        SYNTHESIZE_SCOPE,
        EventKind::SliceSynthesizeStarted {
            slice_name: slice.into(),
        },
        std::future::ready(persist_proceed(
            &synthesized,
            &slice_dir,
            &baseline_index,
            guidance_digest,
        )),
        |out: &(Vec<String>, SnapshotId)| EventKind::SliceSynthesizeCompleted {
            slice_name: slice.into(),
            artifacts: out.0.clone(),
        },
        |err| EventKind::SliceSynthesizeFailed {
            slice_name: slice.into(),
            reason: synthesize_failure_reason(err),
        },
    )
    .await?;

    let tags = validate(layout, now, slice)?;

    slice_actions::transition(&slice_dir, LifecycleStatus::Refined, now)?;

    write_manifest(layout, &plan, &entry, guidance_digest, dependencies, declarations, &slice_dir)?;
    tracing::info!(artifacts = artifacts.len(), "refine completed");

    Ok(RefineOutcome::Refined {
        slice: slice.to_string(),
        artifacts,
        extracted,
        tags,
    })
}

/// Extract fan-out, serially in binding declaration order (the
/// skill's no-parallelism rule). Returns the `(source, lead)` pairs
/// extracted, in binding order.
async fn extract_all<P: Model, S: Source + Workspaces, T: Target, R: Resolver>(
    caps: &super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
    slice: &str, entry: &Entry,
) -> Result<Vec<(String, String)>, Error> {
    let mut extracted = Vec::with_capacity(entry.sources.len());
    for binding in &entry.sources {
        let source = binding.source().to_string();
        let lead = binding.lead(slice).to_string();
        super::extract(
            caps.sources,
            caps.resolver,
            caps.sources,
            paths,
            now,
            &source,
            &lead,
            slice,
        )
        .await?;
        extracted.push((source, lead));
    }
    Ok(extracted)
}

/// Assemble and atomically write the refinement manifest. Runs only
/// after validation and the `refined` transition succeed (RFC-91 D4):
/// an interrupted refinement leaves no manifest for the attempt.
fn write_manifest(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, guidance: SnapshotId,
    dependencies: Vec<Dependency>, declarations: &[BuildInputDeclaration], slice_dir: &Path,
) -> Result<(), Error> {
    let inventory = Leads::load(&layout.leads_path())?;
    let config = ProjectConfig::load(layout.project_dir())?;
    refinement::assemble(
        layout,
        plan,
        entry,
        inventory.leads(),
        refinement::TargetInputs {
            guidance,
            declarations,
            reference: config.adapter.as_deref(),
        },
        dependencies,
    )?
    .write(slice_dir)
}

fn escalated(
    slice: &str, synthesized: crate::judgment::synthesize::Synthesized,
    profile: project::plan::ProfileRef, extracted: Vec<(String, String)>,
) -> RefineOutcome {
    RefineOutcome::Escalated {
        slice: slice.to_string(),
        assessment: synthesized.response.assessment,
        affected: synthesized.response.affected,
        rationale: synthesized.response.rationale.unwrap_or_default(),
        profile,
        extracted,
    }
}

/// Persist a validated `proceed` answer. Guidance digest is already
/// known from the synthesize dispatch.
fn persist_proceed(
    synthesized: &crate::judgment::synthesize::Synthesized, slice_dir: &Path,
    baseline_index: &BaselineIndex, guidance_digest: SnapshotId,
) -> Result<(Vec<String>, SnapshotId), Error> {
    let artifacts = synthesized.response.artifacts.clone().ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-proceed-incomplete",
            "a proceed answer carries the prose artifacts",
            "proceed omitted `artifacts`",
        )
    })?;
    let projected = synthesized.projected.as_ref().ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-proceed-incomplete",
            "a proceed answer projects a model",
            "proceed omitted the projected model",
        )
    })?;
    let written = persist_synthesized(slice_dir, artifacts, projected, baseline_index)?;
    Ok((written, guidance_digest))
}

/// Shape the ordered predecessor pairs into the synthesis inputs'
/// change-local context: digest plus project-relative artifact root.
fn dependency_context(layout: Layout<'_>, dependencies: &[Dependency]) -> Vec<DependencyContext> {
    dependencies
        .iter()
        .map(|dependency| DependencyContext {
            slice: dependency.slice.clone(),
            refinement: dependency.refinement.to_string(),
            artifacts_root: wire_path(layout, &layout.slice_dir(&dependency.slice)),
        })
        .collect()
}

/// Project-relative, `/`-joined form of `path` — the lent-tree path
/// the synthesis inputs envelope hands the agent.
fn wire_path(layout: Layout<'_>, path: &Path) -> String {
    path.strip_prefix(layout.project_dir())
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
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
fn baseline_identity(paths: &ExecutionPaths, entry: &Entry) -> (Vec<Surface>, Vec<Decision>) {
    let layout = Layout::new(paths.project_root());
    let Ok(plan) = Plan::load(&layout.plan_path()) else {
        return Default::default();
    };
    let topology = resolve_topology(&plan);
    let bound = topology.iter().find(|row| row.name == entry.target);
    bound.map(|row| (row.surface.clone(), row.decisions.clone())).unwrap_or_default()
}

fn bound_profile<P: Profiles>(provider: &P, plan: &Plan, entry: &Entry) -> Result<Profile, Error> {
    let table = provider.profiles();
    if let Some(pin) =
        plan.targets.get(&entry.target).and_then(|row| row.model_capability_profile.as_ref())
    {
        return Ok(table.pinned(pin)?.clone());
    }
    Ok(table.resolve()?.clone())
}

/// Warning scope for the best-effort `slice.synthesize.*` brackets.
const SYNTHESIZE_SCOPE: &str = "slice.synthesize";
