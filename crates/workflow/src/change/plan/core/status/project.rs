//! The status projection kernel: plan entries + slice metadata +
//! journal markers → one [`StatusBody`].

use std::path::PathBuf;

use error::Error;

use super::super::model::{Entry, Lifecycle, Plan, Status};
use super::marker::{Marker, newest_marker};
use super::{LoopStep, NextActionKind, StatusBody, StatusCounts, StopBody, StopReason};
use crate::config::Layout;
use crate::slice::{LifecycleStatus, SliceMetadata};

/// Project the read-only `specify plan status` body.
///
/// Selection: the active `in-progress` entry, else the next eligible
/// `pending` entry (what `plan next` would claim), else `drained` /
/// `stop stuck`. For the active entry the journal tail overlays
/// failure classification — the newest marker among that entry's
/// `plan.entry.advanced` / `plan.transition.undone` events and the
/// slice's phase-terminal events decides whether the awaited phase
/// last failed. Pre-claim candidates skip the overlay (nothing has
/// run under the current claim; stale same-name events from earlier
/// plans must not classify).
///
/// `layout` resolves the plan root and the work root: an entry bound
/// to a materialised workspace slot reads that slot's slice metadata
/// and journal, mirroring where phase work writes them.
///
/// # Errors
///
/// Propagates journal I/O failures and a corrupt `metadata.yaml`
/// ([`Error::YamlDe`]); a missing slice directory is the fresh-slice
/// signal, not an error.
pub fn plan_status_body(plan: &Plan, layout: Layout<'_>) -> Result<StatusBody, Error> {
    let counts = StatusCounts {
        pending: count(plan, Status::Pending),
        in_progress: count(plan, Status::InProgress),
        done: count(plan, Status::Done),
    };
    let active = plan.entries.iter().find(|e| e.status == Status::InProgress);

    if plan.lifecycle == Lifecycle::Pending {
        return Ok(assemble(plan, counts, active, Resolution::stop(StopReason::PlanNotApproved)));
    }

    let resolution = match active {
        Some(entry) => resolve_entry(plan, entry, layout, JournalOverlay::Apply)?,
        None => match plan.next_eligible() {
            Some(entry) => resolve_entry(plan, entry, layout, JournalOverlay::Skip)?,
            None if plan.is_drained() => Resolution::drained(),
            None => Resolution::stop(StopReason::Stuck),
        },
    };
    Ok(assemble(plan, counts, active, resolution))
}

/// Whether the journal failure overlay applies to the candidate entry.
/// Only the active `in-progress` entry carries a claim window
/// (`plan.entry.advanced`) that scopes phase-terminal events to the
/// current plan.
#[derive(Clone, Copy, PartialEq, Eq)]
enum JournalOverlay {
    Apply,
    Skip,
}

/// Intermediate projection outcome for one candidate entry.
struct Resolution {
    action: NextActionKind,
    slice: Option<String>,
    project: Option<String>,
    last_completed: Option<LoopStep>,
    stop: Option<StopBody>,
}

impl Resolution {
    const fn stop(reason: StopReason) -> Self {
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

    const fn drained() -> Self {
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

    fn stop_for(
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

fn count(plan: &Plan, status: Status) -> usize {
    plan.entries.iter().filter(|e| e.status == status).count()
}

fn assemble(
    plan: &Plan, counts: StatusCounts, active: Option<&Entry>, resolution: Resolution,
) -> StatusBody {
    let next_action = match (resolution.action, &resolution.slice, &resolution.stop) {
        (NextActionKind::Drained, ..) => "drained".to_string(),
        (NextActionKind::Stop, _, Some(stop)) => format!("stop {}", stop.reason),
        (action, Some(slice), _) => format!("{action} {slice}"),
        // Unreachable by construction: every non-stop, non-drained
        // resolution carries a slice. Render the bare verb if it ever
        // happens rather than panicking in a read-only projection.
        (action, None, _) => action.to_string(),
    };
    StatusBody {
        plan: plan.name.to_string(),
        lifecycle: plan.lifecycle,
        counts,
        active: active.map(|e| e.name.to_string()),
        next_action,
        action: resolution.action,
        current_step: current_step(&resolution),
        last_completed: resolution.last_completed,
        resume: resume_point(plan, &resolution),
        slice: resolution.slice,
        project: resolution.project,
        stop: resolution.stop,
    }
}

/// `current-step`: the phase the targeted slice is at — the
/// dispatched phase, or the phase a stop is parked on.
fn current_step(resolution: &Resolution) -> Option<LoopStep> {
    match resolution.action {
        NextActionKind::Refine => Some(LoopStep::Refine),
        NextActionKind::Build => Some(LoopStep::Build),
        NextActionKind::Merge => Some(LoopStep::Merge),
        NextActionKind::Drained => None,
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed => Some(LoopStep::Refine),
            StopReason::BuildFailed => Some(LoopStep::Build),
            // `merge-incomplete` parks inside merge: the spec merge
            // landed but the per-entry `done` stamp — merge's last
            // sub-step — has not.
            StopReason::MergeConflict | StopReason::MergeIncomplete => Some(LoopStep::Merge),
            StopReason::PlanNotApproved | StopReason::SliceDropped | StopReason::Stuck => None,
        }),
    }
}

/// `resume`: the next valid resume point as a literal command.
/// `None` when no single command makes progress.
fn resume_point(plan: &Plan, resolution: &Resolution) -> Option<String> {
    let slice = resolution.slice.as_deref();
    match resolution.action {
        NextActionKind::Refine => slice.map(|s| format!("/spec:refine {s}")),
        NextActionKind::Build => slice.map(|s| format!("/spec:build {s}")),
        NextActionKind::Merge => slice.map(|s| format!("/spec:merge {s}")),
        NextActionKind::Drained => Some(format!("/spec:finalize {}", plan.name)),
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::PlanNotApproved => {
                Some(format!("specify plan transition {} approved", plan.name))
            }
            StopReason::RefineFailed => slice.map(|s| format!("/spec:refine {s}")),
            StopReason::BuildFailed => slice.map(|s| format!("/spec:build {s}")),
            StopReason::MergeConflict => slice.map(|s| format!("/spec:merge {s}")),
            StopReason::MergeIncomplete => {
                slice.map(|s| format!("specify plan transition {s} done"))
            }
            StopReason::SliceDropped | StopReason::Stuck => None,
        }),
    }
}

/// Dispatch one candidate entry: slot-aware slice lifecycle first,
/// then (for the active entry) the journal failure overlay.
fn resolve_entry(
    plan: &Plan, entry: &Entry, layout: Layout<'_>, overlay: JournalOverlay,
) -> Result<Resolution, Error> {
    let work_root = resolve_work_root(layout, entry);
    let work_layout = Layout::new(&work_root);
    let slice_dir = work_layout.slices_dir().join(entry.name.as_str());

    let lifecycle = match SliceMetadata::load(&slice_dir) {
        Ok(metadata) => Some(metadata.status),
        Err(Error::ArtifactNotFound { .. }) => None,
        Err(err) => return Err(err),
    };

    let marker = match overlay {
        JournalOverlay::Apply => newest_marker(work_layout, &plan.name, &entry.name)?,
        JournalOverlay::Skip => None,
    };

    // A merge that completed without the entry's `done` stamp is a torn
    // state whatever the slice tree looks like (the directory is
    // archived on merge).
    if matches!(marker, Some(Marker::MergeSucceeded)) {
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

    // Failure overlay: stop only when the newest marker is a failure of
    // the phase the lifecycle is awaiting. A failure of any other phase
    // means the operator already moved the slice past it.
    if let Some(Marker::PhaseFailed { phase, reason }) = marker
        && phase == awaited
    {
        let stop = match awaited {
            NextActionKind::Refine => StopReason::RefineFailed,
            NextActionKind::Build => StopReason::BuildFailed,
            _ => StopReason::MergeConflict,
        };
        return Ok(Resolution::stop_for(stop, Some(reason), entry, last_completed));
    }

    Ok(Resolution::phase(awaited, entry, last_completed))
}

/// Work root for an entry: the materialised workspace slot
/// (`<plan-root>/workspace/<project>/`) when the entry is
/// project-bound and the slot exists, else the project root. Mirrors
/// the workspace routing under which phase work wrote the slice tree
/// and journal.
fn resolve_work_root(layout: Layout<'_>, entry: &Entry) -> PathBuf {
    if let Some(project) = &entry.project {
        let slot = layout.project_dir().join("workspace").join(project);
        if slot.is_dir() {
            return slot;
        }
    }
    layout.project_dir().to_path_buf()
}
