//! Transactional multi-class merge + archive (`commit`), plus the
//! no-write `preview` variant and the `conflict_check` baseline drift
//! detector. The filesystem is only touched after every delta validates.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::Serialize;
use specify_error::Error;

use crate::merge::artifact_class::{ArtifactClass, MergeStrategy};
use crate::merge::engine::MergeResult;
use crate::slice::{
    LifecycleStatus, Outcome, OutcomeKind, SliceMetadata, SpecKind, TargetOperation, actions,
};

mod parse;
mod read;
mod write;

use parse::system_time_to_utc;
use read::{
    COMPOSITION_FILENAME, check_opaque_drift, composition_overwrite_gate, first_three_way,
    plan_three_way, preview_opaque,
};
use write::{build_merge_summary, commit_opaque, write_three_way_baselines};

/// One 3-way merged spec entry kept in memory by both
/// [`preview`] and [`commit`].
///
/// `class_name` carries the originating
/// [`ArtifactClass::name`] so callers can group results without the
/// engine having to know any per-domain vocabulary. `name` is the spec
/// (or composition) identifier within that class.
///
/// The `Serialize` derive is the wire shape used by `slice merge run`
/// and `slice merge preview`: `class_name` is a routing tag for the
/// CLI's `filter().map()` step (skipped on the wire), `baseline_path`
/// flows through `Path::display`, and `result` is flattened so the
/// envelope exposes only `operations` (the merged text travels to
/// disk via the commit writer, never to JSON callers).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergePreviewEntry {
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
    pub specs: Vec<MergePreviewEntry>,
    /// `DEC-NNNN` ids promoted by this merge, in slug order.
    pub decisions: Vec<String>,
}

impl std::ops::Deref for MergeCommit {
    type Target = [MergePreviewEntry];

    fn deref(&self) -> &Self::Target {
        &self.specs
    }
}

/// One opaque-replace file pre-image discovered under a
/// [`MergeStrategy::OpaqueReplace`] class's `staged_dir`.
#[derive(Debug, Clone)]
pub struct OpaquePreviewEntry {
    /// Originating artefact class name (e.g. `"contracts"`).
    pub class_name: String,
    /// Path relative to the class's `staged_dir`
    /// (e.g. `schemas/user.yaml`).
    pub relative_path: String,
    /// Whether this file already exists in the baseline.
    pub action: OpaqueAction,
}

/// Whether an opaque-replace file is new or replaces an existing
/// baseline file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpaqueAction {
    /// New file — no corresponding baseline file exists.
    Added,
    /// Replacement — a baseline file at the same path will be overwritten.
    Replaced,
}

/// Complete preview of a slice merge: 3-way merge entries grouped by
/// class plus opaque-replace pre-images grouped by class.
#[derive(Debug, Clone)]
#[must_use]
pub struct PreviewResult {
    /// 3-way merge entries (one per spec/composition per
    /// `ThreeWayMerge` class). Sorted by `(class_name, name)`.
    pub three_way: Vec<MergePreviewEntry>,
    /// Opaque-replace pre-images (one per file per `OpaqueReplace`
    /// class). Sorted by `(class_name, relative_path)`.
    pub opaque: Vec<OpaquePreviewEntry>,
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
    #[serde(with = "specify_error::serde_rfc3339")]
    pub baseline_modified_at: Timestamp,
}

/// Dry-run of the multi-class merge.
///
/// Computes every in-memory [`MergePreviewEntry`] plus runs the
/// baseline coherence validator on each merged output, **without**
/// writing baselines, transitioning status, or archiving. Also reports
/// every file that would be promoted by an
/// [`MergeStrategy::OpaqueReplace`] class.
///
/// Unlike [`commit`] this does not gate on
/// `LifecycleStatus::Built` — the refine / build / merge skill pipeline
/// previews while the slice is still `refined` or `built` so the
/// human can confirm operations before the merge skill commits.
///
/// # Errors
///
/// - [`Error::Diag { code: "merge-spec-conflicts" }`] aggregating every
///   per-spec merge conflict and post-merge `validate_baseline` failure
///   into a single newline-joined detail string.
/// - [`Error::Filesystem`] (`op = "readdir" | "dir-entry" | "path-prefix"
///   | "file-type" | "read"`) for directory-walk and per-file I/O
///   failures while scanning the staged trees and reading deltas /
///   baselines.
/// - [`Error::Diag { code: "merge-non-utf8-name" }`] for the rare
///   non-I/O failure that has no `Error::Filesystem` op equivalent.
/// - Whatever [`Error`] the inner [`crate::merge::engine::merge`] or
///   [`crate::merge::composition::merge`] surfaces, propagated unchanged.
pub fn preview(slice_dir: &Path, classes: &[ArtifactClass]) -> Result<PreviewResult, Error> {
    let three_way = plan_three_way(slice_dir, classes)?;
    let opaque = preview_opaque(classes)?;
    Ok(PreviewResult { three_way, opaque })
}

/// Atomic multi-class merge plus archive.
///
/// Gates on `LifecycleStatus::Built`, runs [`preview`]'s
/// in-memory plan, writes each merged baseline, transitions status to
/// `Merged` with `merged_at`/`completed_at` timestamps, stamps an
/// `Outcome { phase: Merge, outcome: Success }` into
/// `metadata.yaml`, then archives the slice directory via
/// `crate::slice::actions::archive`.
///
/// The outcome stamp is written atomically with the status transition,
/// before the archive move. This ensures the archived `metadata.yaml`
/// carries the merge-success outcome so that `/spec:execute` can read
/// it via direct `metadata.yaml` inspection (falling back to the archive
/// when the active slice directory no longer exists).
///
/// `now` records the `merged_at`, `completed_at`, and outcome stamp;
/// dispatchers pass `Timestamp::now` and tests pin a deterministic value.
///
/// `allow_composition_replace` authorises a whole-document (`screens:`)
/// slice composition to overwrite a non-empty baseline. It threads no
/// further than this function: the A3 `composition_overwrite_gate`
/// precondition reads it directly, and it never reaches the pure merge
/// kernel (composition overwrite-gate placement).
///
/// # Errors
///
/// - [`Error::Diag`] with `code = "lifecycle"` when the slice's status
///   is not [`LifecycleStatus::Built`] on entry, or when the
///   `Built → Merged` transition is rejected (e.g. terminal-state
///   re-entry).
/// - [`Error::Diag { code: "composition-baseline-overwrite-blocked" }`]
///   when the slice composition would overwrite a non-empty baseline
///   without `allow_composition_replace`.
/// - Every error documented on [`preview`] (the in-memory plan
///   is computed before any writes).
/// - [`Error::Filesystem`] (`op = "mkdir" | "copy"`) when the commit
///   phase fails to create a parent directory or copy an opaque-replace
///   file.
/// - [`Error::Diag { code: "merge-write-baseline-failed" }`] when the
///   commit phase fails to write a merged baseline.
/// - [`Error::Diag { code: "merge-archive-failed" }`] when the archive
///   move fails after metadata has already been flipped.
/// - Whatever atomic-write [`Error`] [`SliceMetadata::save`] surfaces
///   (`Error::Io`, `Error::YamlSer`).
pub fn commit(
    slice_dir: &Path, classes: &[ArtifactClass], archive_dir: &Path, now: Timestamp,
    allow_composition_replace: bool,
) -> Result<MergeCommit, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    if metadata.status != LifecycleStatus::Built {
        return Err(Error::Diag {
            code: "lifecycle",
            detail: format!("expected Built, found {:?}", metadata.status),
        });
    }

    // A3 precondition: enforced before any merge work, beside the
    // `Built` gate. Threads the override exactly this far — it never
    // reaches `plan_three_way` or the pure composition kernel.
    if let Some(class) = first_three_way(classes) {
        composition_overwrite_gate(slice_dir, class, allow_composition_replace)?;
    }

    let merged = plan_three_way(slice_dir, classes)?;

    // Decisions pass — core (runs for every target), part of the
    // same merge. Promotion is keyed on the slice name and writes into
    // `.specify/decisions/`, derived from `archive_dir` (a sibling under
    // `.specify/`). The supersede-orphan re-check aborts here before any
    // baseline write, so a failure leaves the slice `Built` for a clean
    // retry; the kernel's `(slice, slug)` guard keeps that retry from
    // double-promoting.
    let decisions = promote_decisions(slice_dir, archive_dir, now)?;

    write_three_way_baselines(&merged)?;
    let opaque_counts = commit_opaque(classes)?;

    metadata.status = metadata.status.transition(LifecycleStatus::Merged)?;
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
        summary: build_merge_summary(&merged, &opaque_counts),
        context: None,
    });
    metadata.save(slice_dir)?;

    actions::archive(slice_dir, archive_dir, now).map_err(|err| Error::Diag {
        code: "merge-archive-failed",
        detail: format!("archive move failed: {err}"),
    })?;

    let mut output: Vec<MergePreviewEntry> = merged;
    output.sort_by(|a, b| {
        (a.class_name.as_str(), a.name.as_str()).cmp(&(b.class_name.as_str(), b.name.as_str()))
    });
    Ok(MergeCommit {
        specs: output,
        decisions,
    })
}

/// Promote the slice's Decision Records into the baseline catalogue.
/// `archive_dir` is `<project>/.specify/archive`, so its grandparent is
/// the project root and its parent is `.specify`; the catalogue lives at
/// `.specify/decisions/`. The slice name is the slice directory's final
/// component.
fn promote_decisions(
    slice_dir: &Path, archive_dir: &Path, now: Timestamp,
) -> Result<Vec<String>, Error> {
    let Some(project_dir) = archive_dir.parent().and_then(Path::parent) else {
        return Ok(Vec::new());
    };
    let slice_name = slice_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    crate::decisions::promote(slice_dir, project_dir, slice_name, now)
}

/// Check for baseline drift on the modified `touched_specs` and on
/// every staged opaque-replace file.
///
/// For each `type: modified` `touched_spec`, the check reports whether
/// the corresponding baseline file under each
/// [`MergeStrategy::ThreeWayMerge`] class has been modified after the
/// slice's `defined_at` timestamp. For each staged file under a
/// [`MergeStrategy::OpaqueReplace`] class, the check reports the same
/// drift against the matching baseline file.
///
/// Returns an empty `Vec` when nothing is stale, the slice has no
/// `touched_specs`, or `defined_at` is missing (in which case the call
/// is a silent no-op — the merge skill should refuse to proceed until
/// define has run).
///
/// # Errors
///
/// - [`Error::Diag { code: "merge-defined-at-malformed" }`] when the
///   slice's `defined_at` stamp is present but does not parse as
///   rfc3339.
/// - [`Error::Diag { code: "merge-mtime-out-of-range" }`] when a baseline
///   mtime cannot be converted to a UTC `jiff::Timestamp`.
/// - [`Error::Io`] when a baseline file's metadata cannot be read for
///   any reason other than `NotFound` (a missing baseline for a
///   `type: modified` entry is treated as a declaration mismatch and
///   silently skipped).
/// - [`Error::Filesystem`] (`op = "readdir" | "dir-entry" | "path-prefix"`)
///   while walking opaque-replace staged trees.
/// - Whatever [`Error`] [`SliceMetadata::load`] surfaces.
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
