//! The status projection: plan entries + the shared execution-state
//! kernel → one [`StatusBody`].

use error::Error;

use super::super::execution::{JournalOverlay, Resolution, resolve_entry};
use super::super::model::{Entry, Lifecycle, Plan, Status};
use super::{LoopStep, NextActionKind, StatusBody, StatusCounts, StopReason};
use crate::config::Layout;

/// Project the read-only `emery plan status` body.
///
/// Selection: the active `in-progress` entry, else the next eligible
/// `pending` entry (what `plan next` would claim), else `drained` /
/// `stop stuck`. The per-entry decision — slot-aware slice lifecycle
/// plus (for the active entry) the folded claim-window journal facts —
/// is the shared `resolve_entry` execution kernel; pre-claim
/// candidates skip the journal overlay (nothing has run under the
/// current claim; stale same-name events from earlier plans must not
/// classify).
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
        NextActionKind::Refine => slice.map(|s| format!("/emery:refine {s}")),
        NextActionKind::Build => slice.map(|s| format!("/emery:build {s}")),
        NextActionKind::Merge => slice.map(|s| format!("/emery:merge {s}")),
        NextActionKind::Drained => Some(format!("/emery:finalize {}", plan.name)),
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::PlanNotApproved => Some("emery plan approve".to_string()),
            StopReason::RefineFailed => slice.map(|s| format!("/emery:refine {s}")),
            StopReason::BuildFailed => slice.map(|s| format!("/emery:build {s}")),
            StopReason::MergeConflict => slice.map(|s| format!("/emery:merge {s}")),
            StopReason::MergeIncomplete => {
                slice.map(|s| format!("emery plan transition {s} done"))
            }
            StopReason::SliceDropped | StopReason::Stuck => None,
        }),
    }
}
