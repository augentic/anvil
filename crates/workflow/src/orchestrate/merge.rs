//! Deterministic guest merge orchestrator.

use std::path::PathBuf;

use jiff::Timestamp;
use specify_error::Error;

use crate::change::{Plan, Status};
use crate::config::{Layout, with_state};
use crate::journal::{self, EventKind};
use crate::merge::{ArtifactClass, MergeCommit, MergePreviewEntry, slice as slice_merge};

/// The result of a completed guest [`merge`].
#[derive(Debug, Clone)]
pub struct MergeOutcome {
    /// The 3-way merged spec/composition entries.
    pub merged: Vec<MergePreviewEntry>,
    /// `DEC-NNNN` ids promoted into the Decision Record catalogue.
    pub decisions: Vec<String>,
    /// Where the slice's working directory was archived.
    pub archive_path: PathBuf,
}

/// Merge one built slice into the baseline and stamp its plan entry
/// `done`.
///
/// The guest collapse of `specify slice merge run`, deterministic-only
/// (no target merge brief is ever dispatched, so there is no
/// `seam.merge`): the `slice.merge.*` pair
/// brackets [`slice_merge::commit`], the workspace-clone git commit
/// leg is skipped with an explicit `slice.merge.commit-skipped` event
/// (the guest owns no git surface; lifecycle authority is `.specify/`
/// state), the durable `slice.archive.created` ledger entry lands with
/// no `merge-sha`, and the plan entry stamps `done`. No plan-lock gate:
/// the lock fences separate OS processes racing the plan, and the
/// guest collapses every breakout in-process — this assumes the guest
/// is the sole `.specify/` writer during a run (non-concurrent stack
/// use is the documented coexistence rule).
///
/// `classes` is the artifact-class set the deltas merge under (the
/// caller resolves it, mirroring how the native verb owns the omnia
/// default set).
///
/// # Errors
///
/// - propagates the `lifecycle` gate, validator, and apply failures
///   from [`slice_merge::commit`].
/// - `plan-entry-not-found` / transition failures from the `done`
///   stamp (skipped silently when no `plan.yaml` exists, matching
///   native standalone merges).
pub fn merge(
    layout: Layout<'_>, now: Timestamp, slice: &str, classes: &[ArtifactClass],
    allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeStarted {
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    match commit_run(layout, now, slice, classes, allow_composition_replace) {
        Ok(outcome) => {
            journal::emit_best_effort(
                layout,
                now,
                EventKind::SliceMergeSucceeded {
                    slice_name: slice.into(),
                },
                "slice.merge",
            );
            Ok(outcome)
        }
        Err(err) => {
            journal::emit_best_effort(
                layout,
                now,
                EventKind::SliceMergeFailed {
                    slice_name: slice.into(),
                    reason: err.variant_str().into_owned(),
                },
                "slice.merge",
            );
            Err(err)
        }
    }
}

/// Validator + apply core: commit the deltas, journal the skipped git
/// leg, append the outcome-ledger entry, and stamp the plan entry
/// `done`. Wrapped by [`merge`] so the `slice.merge.*` pair brackets
/// it.
fn commit_run(
    layout: Layout<'_>, now: Timestamp, slice: &str, classes: &[ArtifactClass],
    allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    let slice_dir = layout.slices_dir().join(slice);
    let archive_dir = layout.archive_dir();

    let merged =
        slice_merge::commit(&slice_dir, classes, &archive_dir, now, allow_composition_replace)?;

    // D2: the guest owns no git surface, so the native workspace-clone
    // auto-commit leg is skipped — explicitly, so a journal reader can
    // tell a guest merge from a native merge that ran outside a clone.
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeCommitSkipped {
            slice_name: slice.into(),
        },
        "slice.merge",
    );

    emit_archive_created(layout, now, slice, &merged);

    stamp_plan_entry_done(layout, slice)?;

    let today = now.strftime("%Y-%m-%d").to_string();
    let archive_path = archive_dir.join(format!("{today}-{slice}"));
    Ok(MergeOutcome {
        merged: merged.specs,
        decisions: merged.decisions,
        archive_path,
    })
}

/// Append the durable `slice.archive.created` outcome-ledger entry.
/// `merge-sha` stays absent — there is no git leg in-guest.
/// Best-effort: a journal-write failure must not undo a committed
/// merge.
fn emit_archive_created(layout: Layout<'_>, now: Timestamp, slice: &str, merged: &MergeCommit) {
    let touched_specs: Vec<String> = merged.specs.iter().map(|e| e.name.clone()).collect();
    let outcome_summary = if merged.specs.is_empty() {
        "no baseline specs touched".to_string()
    } else {
        merged
            .specs
            .iter()
            .map(|e| {
                format!("{}: {}", e.name, crate::merge::summarise_operations(&e.result.operations))
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceArchiveCreated {
            slice_name: slice.into(),
            touched_specs,
            outcome_summary,
            merge_sha: None,
            decisions: merged.decisions.clone(),
        },
        "slice.archive.created",
    );
}

/// workflow §Workflow: the merge step is the sole writer of per-entry
/// `done`. Standalone merges without `plan.yaml` skip this step
/// silently, matching the native verb.
fn stamp_plan_entry_done(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        if !plan.entries.iter().any(|e| e.name == slice) {
            return Err(Error::Diag {
                code: "plan-entry-not-found",
                detail: format!("no slice named '{slice}' in plan"),
            });
        }
        plan.transition(slice, Status::Done)?;
        Ok(())
    })?;
    Ok(())
}
