//! Transactional multi-class merge + archive (`commit`), plus the
//! `conflict_check` baseline drift detector. The filesystem is only
//! touched after every delta validates.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::adapter::TargetOperation;
use serde::Serialize;

use crate::merge::artifact_class::{ArtifactClass, MergeStrategy};
use crate::merge::engine::MergeResult;
use crate::{Outcome, OutcomeKind, SliceMetadata, SpecKind, actions};

#[cfg(test)]
mod goldens;
mod parse;
mod read;
mod write;

use parse::system_time_to_utc;
use read::{COMPOSITION_FILENAME, check_opaque_drift, first_three_way, overwrite_gate, three_way};
use write::{commit_opaque, summary, write_baselines};

/// One 3-way merged spec entry kept in memory by the merge plan and
/// [`commit`].
///
/// `class_name` is a routing tag for the CLI's grouping step (skipped
/// on the wire); `result` is flattened so the envelope exposes only
/// `operations` — the merged text travels to disk via the commit
/// writer, never to JSON callers.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreviewEntry {
    /// Originating artefact class name (e.g. `"specs"`).
    #[serde(skip)]
    pub class_name: String,
    /// Spec/composition name (e.g. `"login"`, `"composition"`).
    pub name: String,
    /// Absolute path where the merged baseline will be written.
    #[serde(serialize_with = "ser_baseline_path")]
    pub baseline_path: PathBuf,
    /// In-memory merge result.
    #[serde(flatten)]
    pub result: MergeResult,
}

#[expect(clippy::ptr_arg, reason = "serde `serialize_with` requires `&PathBuf`")]
fn ser_baseline_path<S: serde::Serializer>(v: &PathBuf, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(&v.display())
}

/// Outcome of a [`commit`].
///
/// Carries the merged spec entries plus the `DEC-NNNN` ids promoted into
/// the Decision Record catalogue (empty when the slice authored
/// none). Derefs to the spec slice so the common `merged.iter()` /
/// `merged.len()` / `merged[i]` callers keep working unchanged.
#[derive(Debug, Clone)]
pub struct MergeCommit {
    /// The 3-way merged spec/composition entries.
    pub specs: Vec<PreviewEntry>,
    /// `DEC-NNNN` ids promoted by this merge, in slug order.
    pub decisions: Vec<String>,
}

impl std::ops::Deref for MergeCommit {
    type Target = [PreviewEntry];

    fn deref(&self) -> &Self::Target {
        &self.specs
    }
}

/// One-line human summary of a merged entry's operations, e.g.
/// `2 added, 1 modified` or `created baseline with 4 requirement(s)`.
///
/// Shared by the merge phase's output and the
/// `slice.archive.created` outcome-ledger summary, so the ledger text
/// never drifts between paths.
#[must_use]
pub fn summarise_operations(ops: &[crate::merge::MergeOperation]) -> String {
    use crate::merge::MergeOperation;
    let mut counts: [(u32, &str); 4] =
        [(0, "added"), (0, "modified"), (0, "removed"), (0, "renamed")];
    let mut created_baseline = None;
    for op in ops {
        match op {
            MergeOperation::Added { .. } => counts[0].0 += 1,
            MergeOperation::Modified { .. } => counts[1].0 += 1,
            MergeOperation::Removed { .. } => counts[2].0 += 1,
            MergeOperation::Renamed { .. } => counts[3].0 += 1,
            MergeOperation::CreatedBaseline { requirement_count } => {
                created_baseline = Some(*requirement_count);
            }
        }
    }
    if let Some(count) = created_baseline {
        return format!("created baseline with {count} requirement(s)");
    }
    let parts: Vec<String> =
        counts.iter().filter(|(c, _)| *c > 0).map(|(c, label)| format!("{c} {label}")).collect();
    if parts.is_empty() { "no-op".to_string() } else { parts.join(", ") }
}

/// One `type: modified` `touched_spec` whose baseline has been modified
/// after the slice's `defined_at` timestamp. The plan skill surfaces
/// this list to the human so they can confirm or abort the merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BaselineConflict {
    /// Adapter (spec directory) name.
    pub adapter: String,
    /// Slice's `defined_at` stamp, copied verbatim from `metadata.yaml`.
    pub defined_at: String,
    /// Baseline file modification time.
    #[serde(with = "project::serde_time::rfc3339")]
    pub baseline_modified_at: Timestamp,
}

/// Atomic multi-class merge plus archive.
///
/// Gates on a fact-substrate build record, writes each merged
/// baseline, stamps the merge outcome into `metadata.yaml`, then
/// archives the slice directory. The outcome stamp is written before
/// the archive move so the archived `metadata.yaml` carries the
/// merge-success outcome the plan execute loop reads.
/// `allow_composition_replace` threads only as far as the
/// `overwrite_gate` precondition — never into the pure merge kernel.
///
/// # Errors
///
/// Lifecycle, overwrite, preview, and write failures occur before the
/// archive move. `merge-archive-failed` means merged metadata and
/// baselines may already be persisted and require operator recovery.
pub fn commit(
    slice_dir: &Path, classes: &[ArtifactClass], archive_dir: &Path, now: Timestamp,
    allow_composition_replace: bool,
) -> Result<MergeCommit, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    // RFC-86 D2 / D27: "built" projects from fact-substrate build
    // records, not stored LifecycleStatus or `build/patch.yaml`.
    if !project::build_record::BuildRecord::present(slice_dir) {
        return Err(Error::Diag {
            code: "slice-lifecycle",
            detail: format!(
                "cannot merge: slice `{}` has no builds/<digest>.yaml (not yet built)",
                slice_dir.file_name().and_then(|s| s.to_str()).unwrap_or("unknown")
            ),
        });
    }

    // A3 precondition: enforced before any merge work, beside the
    // `Built` gate. Threads the override exactly this far — it never
    // reaches `three_way` or the pure composition kernel.
    if let Some(class) = first_three_way(classes) {
        overwrite_gate(slice_dir, class, allow_composition_replace)?;
    }

    let merged = three_way(slice_dir, classes)?;

    // The supersede-orphan re-check aborts before any baseline write,
    // leaving the slice `Built` for a clean retry; the kernel's
    // `(slice, slug)` guard keeps that retry from double-promoting.
    let decisions = promote_decisions(slice_dir, archive_dir, now)?;

    write_baselines(&merged)?;
    let opaque_counts = commit_opaque(classes)?;

    // RFC-86 D2 / D11: no stored lifecycle field; stamp merge times only.
    if metadata.completed_at.is_none() {
        metadata.completed_at = Some(now);
    }
    if metadata.merged_at.is_none() {
        metadata.merged_at = Some(now);
    }
    metadata.outcome = Some(Outcome {
        phase: TargetOperation::Merge,
        kind: OutcomeKind::Success,
        at: now,
        summary: summary(&merged, &opaque_counts),
        context: None,
    });
    metadata.save(slice_dir)?;

    actions::archive(slice_dir, archive_dir, now).map_err(|err| Error::Diag {
        code: "merge-archive-failed",
        detail: format!("archive move failed: {err}"),
    })?;

    let mut output: Vec<PreviewEntry> = merged;
    output.sort_by(|a, b| {
        (a.class_name.as_str(), a.name.as_str()).cmp(&(b.class_name.as_str(), b.name.as_str()))
    });
    Ok(MergeCommit {
        specs: output,
        decisions,
    })
}

/// Promote the slice's Decision Records into the baseline catalogue.
/// `archive_dir` is `<project>/.emery/archive`, so its grandparent is
/// the project root and its parent is `.emery`; the catalogue lives at
/// `.emery/decisions/`. The slice name is the slice directory's final
/// component.
fn promote_decisions(
    slice_dir: &Path, archive_dir: &Path, now: Timestamp,
) -> Result<Vec<String>, Error> {
    let Some(project_dir) = archive_dir.parent().and_then(Path::parent) else {
        return Ok(Vec::new());
    };
    let slice_name = slice_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    project::decisions::promote(slice_dir, project_dir, slice_name, now)
}

/// Check for baseline drift on the modified `touched_specs` and on
/// every staged opaque-replace file, against the slice's `defined_at`
/// timestamp.
///
/// Returns an empty `Vec` when nothing is stale, the slice has no
/// `touched_specs`, or `defined_at` is missing — a silent no-op the
/// merge skill should treat as "define has not run".
///
/// # Errors
///
/// Malformed timestamps and filesystem failures retain their diagnostic
/// taxonomy. A missing modified baseline is a declaration mismatch and
/// is skipped rather than reported as drift.
pub fn conflict_check(
    slice_dir: &Path, classes: &[ArtifactClass],
) -> Result<Vec<BaselineConflict>, Error> {
    let metadata = SliceMetadata::load(slice_dir)?;
    let Some(defined_at) = metadata.defined_at else {
        return Ok(Vec::new());
    };
    let defined_raw = defined_at.strftime("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut conflicts: Vec<BaselineConflict> = Vec::new();

    // Touched-spec drift across every ThreeWayMerge class. Multi-class
    // projects surface drift for each baseline that contains a touched
    // spec name.
    for class in classes.iter().filter(|c| matches!(c.strategy, MergeStrategy::ThreeWayMerge)) {
        for touched in &metadata.touched_specs {
            if touched.kind != SpecKind::Modified {
                continue;
            }
            let baseline = class.baseline_dir.join(&touched.name).join("spec.md");
            let meta = match std::fs::metadata(&baseline) {
                Ok(m) => m,
                // A missing baseline for a `type: modified` entry is weird
                // but not a conflict — it's a declaration mismatch for the
                // skill to surface differently. Skip here.
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(Error::Io(err)),
            };
            let mtime = system_time_to_utc(meta.modified()?)?;
            if mtime > defined_at {
                conflicts.push(BaselineConflict {
                    adapter: touched.name.clone(),
                    defined_at: defined_raw.clone(),
                    baseline_modified_at: mtime,
                });
            }
        }
    }

    // Composition drift — the convention is exactly one composition
    // delta per slice, promoted into the first ThreeWayMerge class's
    // baseline.
    let composition_delta = slice_dir.join(COMPOSITION_FILENAME);
    if composition_delta.is_file()
        && let Some(class) = first_three_way(classes)
    {
        let comp_baseline = class.baseline_dir.join(COMPOSITION_FILENAME);
        if let Ok(meta) = std::fs::metadata(&comp_baseline) {
            let mtime = system_time_to_utc(meta.modified()?)?;
            if mtime > defined_at {
                conflicts.push(BaselineConflict {
                    adapter: "composition".to_string(),
                    defined_at: defined_raw.clone(),
                    baseline_modified_at: mtime,
                });
            }
        }
    }

    for class in classes.iter().filter(|c| matches!(c.strategy, MergeStrategy::OpaqueReplace)) {
        if !class.staged_dir.is_dir() {
            continue;
        }
        check_opaque_drift(
            &class.staged_dir,
            &class.staged_dir,
            &class.baseline_dir,
            &class.name,
            &defined_raw,
            defined_at,
            &mut conflicts,
        )?;
    }

    conflicts.sort_by(|a, b| a.adapter.cmp(&b.adapter));
    Ok(conflicts)
}
