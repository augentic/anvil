//! Post-projection synthesis persistence: the render → stage → persist
//! tail run by the guest refine orchestrator ([`crate::orchestrate`]).
//!
//! [`persist_synthesized`] takes an already-projected model (the
//! kernel ran in-guest inside the
//! judgment repair loop) and owns everything after it: the serialised
//! model re-validation, the provenance render into each
//! `specs/<domain>/spec.md`, the Decision Record sidecar render into
//! `decisions/<slug>.md` (exact-set replacement, so stale generated
//! records never survive a re-refine), the `metadata.touched_specs`
//! refresh, and the atomic batch persist. Everything is staged in
//! memory before the first write, so a failure leaves prior artifacts
//! intact.

use std::path::{Path, PathBuf};

use artifacts::decision::{DecisionRecord, DecisionStatus};
use error::{Error, Result};

use crate::model::SliceModel;
use crate::synthesis::baseline::BaselineIndex;
use crate::synthesis::render::render_spec_files;
use crate::synthesis::wire::{SynthesisArtifacts, SynthesisDecision};
use crate::{SliceMetadata, actions as slice_actions};

/// Render, stage, and atomically persist one synthesized slice.
///
/// Returns the slice-relative paths written (in write order):
/// `proposal.md`, each `specs/<domain>/spec.md`, `design.md`,
/// `tasks.md`, each `decisions/<slug>.md`, `model.yaml`,
/// `metadata.yaml`.
///
/// `artifacts` are the agent's prose-only bodies; `projected` is the
/// kernel-projected model (ids, status, winners, and rendered sources
/// already derived). The `decisions/` directory is replaced as an
/// exact set — a re-refine that drops a record also removes its file.
///
/// # Errors
///
/// - propagates model serialisation and the re-parse validation
///   failure of the projected model.
/// - propagates `metadata.yaml` load and the atomic write failures.
pub fn persist_synthesized(
    slice_dir: &Path, artifacts: SynthesisArtifacts, projected: &SliceModel,
    baseline_index: &BaselineIndex,
) -> Result<Vec<String>> {
    // Re-validate the projected model against the schema (the kernel
    // already enforced orphans/cross-refs/grammar; the broader drift
    // suite is `slice validate`'s job). `parse_yaml` validates the
    // serialised document and re-parses it.
    let model_yaml = artifacts::atomic::serialise_yaml(projected)?;
    SliceModel::parse_yaml(&model_yaml)?;

    // Render provenance lines into `spec.md` (in memory).
    let specs = render_spec_files(projected, baseline_index);

    // Stage every artifact before the first write so a failure above
    // leaves the prior artifacts intact.
    let mut staged: Vec<StagedFile> = Vec::new();
    staged.push(staged_file(slice_dir, "proposal.md", artifacts.proposal.into_bytes()));
    for spec in &specs {
        let rel = format!("specs/{}/spec.md", spec.domain);
        staged.push(staged_file(slice_dir, &rel, spec.content.clone().into_bytes()));
    }
    staged.push(staged_file(slice_dir, "design.md", artifacts.design.into_bytes()));
    staged.push(staged_file(slice_dir, "tasks.md", artifacts.tasks.into_bytes()));
    for decision in &artifacts.decisions {
        let rel = format!("decisions/{}.md", decision.slug);
        staged.push(staged_file(slice_dir, &rel, render_decision(decision)?.into_bytes()));
    }
    staged.push(staged_file(slice_dir, "model.yaml", model_yaml.into_bytes()));

    let touched = slice_actions::touched_from_rendered(&specs, baseline_index);
    let mut metadata = SliceMetadata::load(slice_dir)?;
    metadata.touched_specs = touched;
    let metadata_yaml = artifacts::atomic::serialise_yaml(&metadata)?;
    staged.push(staged_file(slice_dir, "metadata.yaml", metadata_yaml.into_bytes()));

    // Exact-set replacement: the response's `decisions[]` is the
    // whole set, so any previously generated record not in it is
    // removed before the batch persist (an empty response clears the
    // directory).
    replace_decisions_dir(slice_dir, &artifacts.decisions)?;

    // Persist every staged artifact in one batch.
    let mut written = Vec::with_capacity(staged.len());
    for file in &staged {
        artifacts::atomic::bytes_write(&file.abs, &file.bytes)?;
        written.push(file.rel.clone());
    }

    Ok(written)
}

/// Remove every `decisions/*.md` not named by the response's
/// `decisions[]` set. The staged batch then rewrites the survivors, so
/// re-refine output is an exact projection of the latest response.
fn replace_decisions_dir(slice_dir: &Path, decisions: &[SynthesisDecision]) -> Result<()> {
    let dir = slice_dir.join("decisions");
    if !dir.is_dir() {
        return Ok(());
    }
    let keep: std::collections::BTreeSet<String> =
        decisions.iter().map(|d| format!("{}.md", d.slug)).collect();
    for path in project::decisions::list_md_files(&dir)? {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !keep.contains(name) {
            std::fs::remove_file(&path).map_err(|source| Error::Filesystem {
                op: "remove",
                path: path.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

/// Render one slice-authored Decision Record: the slice-authored
/// front-matter (engine-stamped fields absent) plus the Nygard body.
/// The shape round-trips [`artifacts::decision::parse_decision`], so
/// the refine gate and merge promotion consume it unchanged.
fn render_decision(decision: &SynthesisDecision) -> Result<String> {
    let record = DecisionRecord {
        id: None,
        slug: decision.slug.clone(),
        status: match decision.status {
            DecisionStatus::Rejected => DecisionStatus::Rejected,
            // `superseded` is engine-only and refused by the answer
            // schema; normalise defensively to `accepted`.
            DecisionStatus::Accepted | DecisionStatus::Superseded => DecisionStatus::Accepted,
        },
        slice: None,
        date: None,
        supersedes: decision.supersedes.clone(),
        related: decision.related.clone(),
        topics: decision.topics.clone(),
        superseded_by: None,
    };
    let yaml = artifacts::atomic::serialise_yaml(&record)?;
    Ok(format!(
        "---\n{yaml}---\n\n# {}\n\n## Context\n\n{}\n\n## Decision\n\n{}\n\n## Consequences\n\n{}\n",
        decision.title,
        decision.context.trim(),
        decision.decision.trim(),
        decision.consequences.trim()
    ))
}

/// One artifact staged in memory before the persist loop.
struct StagedFile {
    /// Slice-relative path recorded on the `completed` journal event.
    rel: String,
    /// Absolute path the bytes are written to.
    abs: PathBuf,
    /// File contents.
    bytes: Vec<u8>,
}

/// Build a [`StagedFile`] under `slice_dir` from a slice-relative path.
fn staged_file(slice_dir: &Path, rel: &str, bytes: Vec<u8>) -> StagedFile {
    StagedFile {
        rel: rel.to_string(),
        abs: slice_dir.join(rel),
        bytes,
    }
}

/// Short failure reason / finding code for the `slice.synthesize.failed`
/// journal event payload.
#[must_use]
pub fn failure_reason(err: &Error) -> String {
    match err {
        Error::Validation { code, .. } => code.to_string(),
        Error::Diag { code, .. } => (*code).to_string(),
        other => other.to_string(),
    }
}
