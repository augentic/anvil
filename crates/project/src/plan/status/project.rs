//! The status projection: plan topology + artifacts + the fact union
//! → one [`StatusBody`] (RFC-86 D2).

use std::ops::ControlFlow;

use error::Error;

use super::super::execution::{
    JournalOverlay, Resolution, collect_events, next_eligible, project_ladders, resolve_entry,
    scan_union,
};
use super::super::model::{Entry, Plan, Status};
use super::{LoopStep, NextActionKind, StatusBody, StatusCounts, StopReason};
use crate::config::Layout;
use crate::journal::{Event, EventKind};
use crate::name::SliceName;

/// Project the read-only `emery plan status` body.
///
/// Selection: the first projected `in-progress` entry, else sticky
/// `merge-postflight-failed` when the newest plan-scoped postflight
/// debt event is unacked, else the next eligible projected `pending`
/// entry (what `plan advance` would advance), else `drained` /
/// `stop stuck`. The per-entry decision — artifact phase plus (for
/// an in-progress entry) the folded active-window journal facts — is
/// the shared `resolve_entry` execution kernel; not-yet-advanced
/// candidates skip the journal overlay (nothing has run under the
/// current activation; stale same-name events from earlier plans must
/// not classify).
///
/// Ladder labels and the awaited phase are computed from the fact
/// union and slice artifacts. Stored `Entry.status` /
/// `LifecycleStatus` fields are not read.
///
/// `layout` resolves the plan root and the work root: an entry bound
/// to a materialised workspace slot reads that slot's slice artifacts
/// and journal, mirroring where phase work writes them.
///
/// # Errors
///
/// Propagates journal I/O failures and a corrupt `metadata.yaml`
/// ([`Error::YamlDe`]); a missing slice directory is the fresh-slice
/// signal, not an error.
pub fn plan_status_body(plan: &Plan, layout: Layout<'_>) -> Result<StatusBody, Error> {
    let events = collect_events(plan, layout)?;
    let ladders = project_ladders(plan, &events);
    let counts = StatusCounts {
        pending: count(&ladders, Status::Pending),
        in_progress: count(&ladders, Status::InProgress),
        done: count(&ladders, Status::Done),
    };
    let active = plan
        .entries
        .iter()
        .find(|e| ladders.get(&e.name).copied() == Some(Status::InProgress));

    let resolution = match active {
        Some(entry) => resolve_entry(plan, entry, layout, JournalOverlay::Apply, &events)?,
        None => {
            // Sticky postflight debt: after a non-rollback postflight
            // failure the entry is already `done`, so nothing is
            // in-progress — project the stop until execute acknowledges.
            if let Some(debt) = postflight_debt(plan, &events) {
                debt
            } else {
                match next_eligible(plan, &ladders) {
                    Some(entry) => {
                        resolve_entry(plan, entry, layout, JournalOverlay::Skip, &events)?
                    }
                    None if ladders.values().all(|s| *s == Status::Done) => Resolution::drained(),
                    None => Resolution::stop(StopReason::Stuck),
                }
            }
        }
    };
    Ok(assemble(plan, counts, active, &ladders, resolution))
}

/// When the chronologically latest among
/// `{slice.merge.postflight-failed, plan.merge-postflight.acknowledged}`
/// (restricted to slices named in this plan) is a postflight failure,
/// project the sticky `merge-postflight-failed` stop for that slice.
fn postflight_debt(plan: &Plan, events: &[Event]) -> Option<Resolution> {
    let mut resolution = None;
    scan_union(events, |event| {
        match &event.kind {
            EventKind::SliceMergePostflightFailed { slice_name, reason }
                if plan.entries.iter().any(|e| e.name == *slice_name) =>
            {
                let entry = plan.entries.iter().find(|e| e.name == *slice_name);
                if let Some(entry) = entry {
                    resolution = Some(Resolution::stop_for(
                        StopReason::MergePostflightFailed,
                        Some(reason.clone()),
                        entry,
                        Some(LoopStep::Merge),
                    ));
                }
                ControlFlow::Break(())
            }
            EventKind::PlanMergePostflightAcknowledged { slice_name }
                if plan.entries.iter().any(|e| e.name == *slice_name) =>
            {
                resolution = None;
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        }
    });
    resolution
}

fn count(ladders: &std::collections::HashMap<SliceName, Status>, status: Status) -> usize {
    ladders.values().filter(|s| **s == status).count()
}

fn assemble(
    plan: &Plan, counts: StatusCounts, active: Option<&Entry>,
    ladders: &std::collections::HashMap<SliceName, Status>, resolution: Resolution,
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
        counts,
        active: active.map(|e| e.name.to_string()),
        next_action,
        action: resolution.action,
        current_step: current_step(&resolution),
        last_completed: resolution.last_completed,
        resume: resume_point(plan, ladders, &resolution),
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
            // sub-step — has not. Postflight failure is past merge
            // (`done` + archived) — no awaited phase.
            StopReason::MergeConflict | StopReason::MergeIncomplete => Some(LoopStep::Merge),
            StopReason::MergePostflightFailed | StopReason::SliceDropped | StopReason::Stuck => {
                None
            }
        }),
    }
}

/// `resume`: the next valid resume point as a literal command.
/// `None` when no single command makes progress.
fn resume_point(
    plan: &Plan, ladders: &std::collections::HashMap<SliceName, Status>, resolution: &Resolution,
) -> Option<String> {
    let slice = resolution.slice.as_deref();
    // A fresh plan (no entry has left projected `pending`) resumes
    // through the execute loop, not a phase breakout — the projected
    // `next-action` still names the phase the loop will run first.
    if ladders.values().all(|s| *s == Status::Pending)
        && matches!(
            resolution.action,
            NextActionKind::Refine | NextActionKind::Build | NextActionKind::Merge
        )
    {
        return Some("/emery:execute".to_string());
    }
    match resolution.action {
        NextActionKind::Refine => slice.map(|s| format!("/emery:refine {s}")),
        NextActionKind::Build => slice.map(|s| format!("/emery:build {s}")),
        NextActionKind::Merge => slice.map(|s| format!("/emery:merge {s}")),
        NextActionKind::Drained => Some(format!("/emery:finalize {}", plan.name)),
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed => slice.map(|s| format!("/emery:refine {s}")),
            StopReason::BuildFailed => slice.map(|s| format!("/emery:build {s}")),
            StopReason::MergeConflict => slice.map(|s| format!("/emery:merge {s}")),
            StopReason::MergePostflightFailed => Some("emery plan execute".to_string()),
            StopReason::MergeIncomplete => slice.map(|s| format!("/emery:merge {s}")),
            StopReason::SliceDropped | StopReason::Stuck => None,
        }),
    }
}
