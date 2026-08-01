//! The in-memory execution-state projection: one candidate plan
//! entry, its work-root slice tree, and the folded claim-window
//! journal facts become the next-step decision (refine / build /
//! merge / stop).
//!
//! Shared by the read-only `plan status` projection and (through it)
//! the guest execute loop, so the two surfaces cannot drift. Nothing
//! here is persisted — the projection folds durable state (the plan
//! entry, live slice metadata, journal events) on every call.
//!
//! Journal facts are folded over the entry's **claim window**: the
//! events newer than the entry's most recent `plan.entry.advanced` /
//! `plan.transition.undone`. Within the window, durable merge
//! evidence (`slice.merge.succeeded` / `slice.archive.created`)
//! dominates any later failure marker — a failed retry after a landed
//! merge is noise, and the torn state projects the `merge-incomplete`
//! stop. Otherwise the newest phase terminal decides: a success means
//! "dispatch on lifecycle", a failure of the awaited phase parks the
//! matching stop.

use std::ops::ControlFlow;
use std::path::PathBuf;

use error::Error;

use super::model::{Entry, Plan};
use super::status::{LoopStep, NextActionKind, StopBody, StopReason};
use crate::config::Layout;
use crate::journal::{self, EventKind};
use crate::name::{PlanName, SliceName};
use crate::slice::{LifecycleStatus, SliceMetadata};

/// Whether the claim-window journal overlay applies to the candidate
/// entry. Only the active `in-progress` entry carries a claim window
/// (`plan.entry.advanced`) that scopes phase-terminal events to the
/// current plan; pre-claim candidates skip the overlay so stale
/// same-name events from earlier plans cannot classify.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JournalOverlay {
    Apply,
    Skip,
}

/// The projected next step for one candidate entry.
pub(super) struct Resolution {
    pub action: NextActionKind,
    pub slice: Option<String>,
    pub project: Option<String>,
    pub last_completed: Option<LoopStep>,
    pub stop: Option<StopBody>,
}

impl Resolution {
    pub(super) const fn stop(reason: StopReason) -> Self {
        Self {
            action: NextActionKind::Stop,
            slice: None,
            project: None,
            last_completed: None,
            stop: Some(StopBody {
                reason,
                detail: None,
                hint: reason.hint(),
            }),
        }
    }

    pub(super) const fn drained() -> Self {
        Self {
            action: NextActionKind::Drained,
            slice: None,
            project: None,
            last_completed: None,
            stop: None,
        }
    }

    fn phase(action: NextActionKind, entry: &Entry, last_completed: Option<LoopStep>) -> Self {
        Self {
            action,
            slice: Some(entry.name.to_string()),
            project: entry.project.clone(),
            last_completed,
            stop: None,
        }
    }

    pub(super) fn stop_for(
        reason: StopReason, detail: Option<String>, entry: &Entry, last_completed: Option<LoopStep>,
    ) -> Self {
        Self {
            action: NextActionKind::Stop,
            slice: Some(entry.name.to_string()),
            project: entry.project.clone(),
            last_completed,
            stop: Some(StopBody {
                reason,
                detail,
                hint: reason.hint(),
            }),
        }
    }
}

/// Project one candidate entry: slot-aware slice lifecycle first,
/// then (for the active entry) the folded claim-window journal facts.
pub(super) fn resolve_entry(
    plan: &Plan, entry: &Entry, layout: Layout<'_>, overlay: JournalOverlay,
) -> Result<Resolution, Error> {
    let work_root = resolve_work_root(layout, entry);
    let work_layout = Layout::new(&work_root);
    let slice_dir = work_layout.slice_dir(entry.name.as_str());

    let lifecycle = match SliceMetadata::load(&slice_dir) {
        Ok(metadata) => Some(metadata.status),
        // Both "no slice directory yet" and "directory without
        // metadata.yaml" mean the phase has not created the slice.
        Err(
            Error::ArtifactNotFound { .. }
            | Error::Diag {
                code: "slice-not-found",
                ..
            },
        ) => None,
        Err(err) => return Err(err),
    };

    let facts = match overlay {
        JournalOverlay::Apply => claim_window_facts(work_layout, &plan.name, &entry.name)?,
        JournalOverlay::Skip => ClaimFacts::default(),
    };

    // A merge that completed without the entry's `done` stamp is a torn
    // state whatever the slice tree looks like (the directory is
    // archived on merge). Durable merge evidence dominates any later
    // failure marker in the window.
    if facts.merged {
        return Ok(Resolution::stop_for(
            StopReason::MergeIncomplete,
            None,
            entry,
            Some(LoopStep::Merge),
        ));
    }

    // `last-completed`: the slice lifecycle is the record of the
    // most recent completed step.
    let last_completed = match lifecycle {
        None | Some(LifecycleStatus::Refining | LifecycleStatus::Dropped) => None,
        Some(LifecycleStatus::Refined) => Some(LoopStep::Refine),
        Some(LifecycleStatus::Built) => Some(LoopStep::Build),
        Some(LifecycleStatus::Merged) => Some(LoopStep::Merge),
    };

    let awaited = match lifecycle {
        None | Some(LifecycleStatus::Refining) => NextActionKind::Refine,
        Some(LifecycleStatus::Refined) => NextActionKind::Build,
        Some(LifecycleStatus::Built) => NextActionKind::Merge,
        Some(LifecycleStatus::Dropped) => {
            return Ok(Resolution::stop_for(StopReason::SliceDropped, None, entry, None));
        }
        Some(LifecycleStatus::Merged) => {
            return Ok(Resolution::stop_for(
                StopReason::MergeIncomplete,
                None,
                entry,
                last_completed,
            ));
        }
    };

    // Failure overlay: stop only when the newest terminal is a failure
    // of the phase the lifecycle is awaiting. A failure of any other
    // phase means the operator already moved the slice past it.
    // Durable `archive.created` still marks `merged` above (and wins
    // as `merge-incomplete` for an in-progress entry); postflight debt
    // for a `done` entry is handled plan-wide by `plan_status_body` so
    // it is not shadowed into `merge-incomplete` here.
    if let Some((phase, stop, reason)) = facts.newest_failure
        && phase == awaited
    {
        return Ok(Resolution::stop_for(stop, Some(reason), entry, last_completed));
    }

    Ok(Resolution::phase(awaited, entry, last_completed))
}

/// Work root for an entry: the materialised workspace slot
/// (`<plan-root>/workspace/<project>/`) when the entry is
/// project-bound and the slot exists, else the project root. Mirrors
/// the workspace routing under which phase work wrote the slice tree
/// and journal.
pub(super) fn resolve_work_root(layout: Layout<'_>, entry: &Entry) -> PathBuf {
    if let Some(project) = &entry.project {
        let slot = layout.project_dir().join("workspace").join(project);
        if slot.is_dir() {
            return slot;
        }
    }
    layout.project_dir().to_path_buf()
}

/// The folded journal facts for one entry's claim window.
#[derive(Default)]
struct ClaimFacts {
    /// Durable merge evidence (`slice.merge.succeeded` /
    /// `slice.archive.created`) exists anywhere in the window.
    merged: bool,
    /// The newest phase terminal is a failure of this phase; `None`
    /// when the newest terminal is a success (or none exists).
    /// The [`StopReason`] distinguishes merge-conflict from
    /// merge-postflight-failed when the failure is a merge-phase
    /// terminal.
    newest_failure: Option<(NextActionKind, StopReason, String)>,
}

/// Backward-fold the work root's journal over this entry's claim
/// window: events newer than the newest `plan.entry.advanced` /
/// `plan.transition.undone` for the `(plan, slice)` pair, stopping at
/// that boundary (or the head of the journal).
fn claim_window_facts(
    work_layout: Layout<'_>, plan_name: &PlanName, slice: &SliceName,
) -> Result<ClaimFacts, Error> {
    let mut facts = ClaimFacts::default();
    let mut terminal_seen = false;
    journal::scan_recent(work_layout, |event| {
        match event.kind {
            // The claim boundary: nothing older belongs to this claim.
            EventKind::PlanEntryAdvanced {
                plan_name: p,
                slice_name: s,
            }
            | EventKind::PlanTransitionUndone {
                plan_name: p,
                slice_name: s,
                ..
            } if &p == plan_name && &s == slice => return ControlFlow::Break(()),
            EventKind::SliceMergeSucceeded { slice_name: s } if &s == slice => {
                facts.merged = true;
            }
            EventKind::SliceArchiveCreated { slice_name: s, .. } if &s == slice => {
                facts.merged = true;
            }
            EventKind::SliceSynthesizeCompleted { slice_name: s, .. }
            | EventKind::SliceBuildSucceeded { slice_name: s }
                if &s == slice =>
            {
                terminal_seen = true;
            }
            EventKind::SliceSynthesizeFailed {
                slice_name: s,
                reason,
            } if &s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Refine, StopReason::RefineFailed, reason));
                }
                terminal_seen = true;
            }
            EventKind::SliceBuildFailed {
                slice_name: s,
                reason,
            } if &s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Build, StopReason::BuildFailed, reason));
                }
                terminal_seen = true;
            }
            EventKind::SliceMergeFailed {
                slice_name: s,
                reason,
            } if &s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Merge, StopReason::MergeConflict, reason));
                }
                terminal_seen = true;
            }
            EventKind::SliceMergePostflightFailed {
                slice_name: s,
                reason,
            } if &s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Merge, StopReason::MergePostflightFailed, reason));
                }
                terminal_seen = true;
            }
            _ => {}
        }
        ControlFlow::Continue(())
    })?;
    Ok(facts)
}
