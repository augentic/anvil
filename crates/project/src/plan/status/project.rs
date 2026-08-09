//! The status projection: plan topology + artifacts + the fact union
//! → one [`StatusBody`] (RFC-86 D2 / D22 / D26).

use std::ops::ControlFlow;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;

use super::super::execution::{
    JournalOverlay, Resolution, collect_events, next_eligible, project_ladders, resolve_entry,
    scan_union,
};
use super::super::gaps::plan_gaps_body;
use super::super::in_scope;
use super::super::model::{Entry, Plan, Status};
use super::{LoopStep, NextActionKind, StatusBody, StatusCounts, StopReason};
use crate::config::Layout;
use crate::journal::{Event, EventKind};
use crate::name::SliceName;
use crate::slice::SliceMetadata;

/// Project the read-only `emery plan status` body.
///
/// Selection: first projected `in-progress` entry, else sticky
/// unacked `merge-postflight-failed`, else the next eligible `pending`
/// entry, else `drained` / `stop stuck`. Not-yet-advanced candidates
/// skip the journal overlay — stale same-name events from earlier
/// plans must not classify.
///
/// # Errors
///
/// Propagates journal I/O failures and a corrupt `metadata.yaml`
/// ([`Error::YamlDe`]); a missing slice directory is the fresh-slice
/// signal, not an error.
pub fn plan_status_body(plan: &Plan, layout: Layout<'_>) -> Result<StatusBody, Error> {
    let events = collect_events(layout)?;
    let ladders = project_ladders(plan, &events);
    let counts = StatusCounts {
        pending: count(&ladders, Status::Pending),
        in_progress: count(&ladders, Status::InProgress),
        done: count(&ladders, Status::Done),
    };
    let active =
        plan.entries.iter().find(|e| ladders.get(&e.name).copied() == Some(Status::InProgress));

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
    let gaps = plan_gaps_body(plan, layout)?;
    let all_refined = all_in_scope_refined(plan, layout)?;
    let milestones = Milestones {
        all_refined,
        ready: all_refined && clean_gaps(&gaps),
        authorized: project_authorized(&events),
    };
    Ok(assemble(plan, counts, active, &ladders, resolution, gaps, milestones))
}

/// Plan-wide Ready / Authorized inputs for [`assemble`] (RFC-86 D22).
#[derive(Clone, Copy)]
struct Milestones {
    all_refined: bool,
    ready: bool,
    authorized: bool,
}

/// When the chronologically latest among
/// `{target.merge.wave-postflight-failed, plan.merge-postflight.acknowledged}`
/// (restricted to slices named in this plan) is a postflight failure,
/// project the sticky `merge-postflight-failed` stop for that slice.
fn postflight_debt(plan: &Plan, events: &[Event]) -> Option<Resolution> {
    let mut resolution = None;
    scan_union(events, |event| match &event.kind {
        EventKind::TargetMergeWavePostflightFailed {
            slice_name, reason, ..
        } if plan.entries.iter().any(|e| e.name == *slice_name) => {
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
    });
    resolution
}

fn count(ladders: &std::collections::HashMap<SliceName, Status>, status: Status) -> usize {
    ladders.values().filter(|s| **s == status).count()
}

/// Ready's clean-gap policy: no conflicts and zero open unknowns.
/// Divergence is listed but does not block Ready (D22).
fn clean_gaps(gaps: &super::super::gaps::GapsBody) -> bool {
    !gaps
        .rows
        .iter()
        .any(|row| matches!(row.status, RequirementStatus::Unknown | RequirementStatus::Conflict))
}

/// Every in-scope entry has refined artifacts (model.yaml or spec.md).
/// Empty in-scope set is vacuously refined.
fn all_in_scope_refined(plan: &Plan, layout: Layout<'_>) -> Result<bool, Error> {
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = load_meta(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if !is_refined(&slice_dir) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn load_meta(slice_dir: &Path) -> Result<Option<SliceMetadata>, Error> {
    match SliceMetadata::load(slice_dir) {
        Ok(m) => Ok(Some(m)),
        Err(
            Error::ArtifactNotFound { .. }
            | Error::Diag {
                code: "slice-not-found",
                ..
            },
        ) => Ok(None),
        Err(err) => Err(err),
    }
}

fn is_refined(slice_dir: &Path) -> bool {
    slice_dir.join("model.yaml").is_file() || slice_dir.join("spec.md").is_file()
}

/// Authorized when any `plan.execute.started` fact is in the union.
/// Covering / stale validation lands with the execute writer (S18/S19).
fn project_authorized(events: &[Event]) -> bool {
    events.iter().any(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
}

fn assemble(
    plan: &Plan, counts: StatusCounts, active: Option<&Entry>,
    ladders: &std::collections::HashMap<SliceName, Status>, mut resolution: Resolution,
    gaps: super::super::gaps::GapsBody, milestones: Milestones,
) -> StatusBody {
    // When every in-scope slice is refined but Ready fails, surface
    // review-gaps instead of build (D22). Execute maps ReviewGaps →
    // build; the gap gate enforces waivers / refuses unresolved gaps.
    if milestones.all_refined
        && !milestones.ready
        && matches!(resolution.action, NextActionKind::Build)
    {
        resolution.action = NextActionKind::ReviewGaps;
        resolution.slice = None;
        resolution.project = None;
        resolution.last_completed = Some(LoopStep::Refine);
    }

    let next_action = match (resolution.action, &resolution.slice, &resolution.stop) {
        (NextActionKind::Drained, ..) => "drained".to_string(),
        (NextActionKind::Stop, _, Some(stop)) => format!("stop {}", stop.reason),
        (NextActionKind::ReviewGaps, ..) => "review-gaps".to_string(),
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
        resume: resume_point(plan, ladders, &resolution, &gaps, milestones.ready),
        ready: milestones.ready,
        authorized: milestones.authorized,
        slice: resolution.slice,
        project: resolution.project,
        stop: resolution.stop,
        gaps,
    }
}

/// `current-step`: the phase the targeted slice is at — the
/// dispatched phase, or the phase a stop is parked on.
fn current_step(resolution: &Resolution) -> Option<LoopStep> {
    match resolution.action {
        NextActionKind::Refine => Some(LoopStep::Refine),
        NextActionKind::Build => Some(LoopStep::Build),
        NextActionKind::Merge => Some(LoopStep::Merge),
        NextActionKind::ReviewGaps | NextActionKind::Drained => None,
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed => Some(LoopStep::Refine),
            StopReason::BuildFailed => Some(LoopStep::Build),
            // `merge-incomplete` parks inside merge: the spec merge landed
            // but the per-entry `done` stamp has not. Postflight failure is
            // past merge (`done` + archived) — no awaited phase.
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
    gaps: &super::super::gaps::GapsBody, ready: bool,
) -> Option<String> {
    // A fresh plan (no entry has left projected `pending`) resumes
    // through the execute loop, not a phase breakout. When refined but
    // not Ready, point at waive / gap closure instead.
    if ladders.values().all(|s| *s == Status::Pending)
        && matches!(
            resolution.action,
            NextActionKind::Refine
                | NextActionKind::Build
                | NextActionKind::Merge
                | NextActionKind::ReviewGaps
        )
    {
        return Some(fresh_plan_resume(gaps, ready, resolution.action));
    }
    match resolution.action {
        // Every phase resumes through the execute loop — there are no
        // phase-breakout verbs (RFC-86 three-verb surface).
        NextActionKind::Refine | NextActionKind::Build | NextActionKind::Merge => {
            Some("emery plan execute".to_string())
        }
        NextActionKind::ReviewGaps => Some(gap_resume(gaps)),
        NextActionKind::Drained => Some(format!("/emery:finalize {}", plan.name)),
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed
            | StopReason::BuildFailed
            | StopReason::MergeConflict
            | StopReason::MergePostflightFailed
            | StopReason::MergeIncomplete => Some("emery plan execute".to_string()),
            StopReason::SliceDropped | StopReason::Stuck => None,
        }),
    }
}

/// Post-author / all-pending resume (D26 / D22).
fn fresh_plan_resume(
    gaps: &super::super::gaps::GapsBody, ready: bool, action: NextActionKind,
) -> String {
    if action == NextActionKind::ReviewGaps || (!ready && action == NextActionKind::Build) {
        return gap_resume(gaps);
    }
    // Unrefined or Ready: resume at execute (D26).
    "/emery:execute".to_string()
}

/// Resume when the change is not Ready: conflicts → fix inputs and
/// re-execute (drifted pins re-refine under the epoch); unknowns-only
/// → execute with per-req `--waive`.
fn gap_resume(gaps: &super::super::gaps::GapsBody) -> String {
    if gaps.rows.iter().any(|r| r.status == RequirementStatus::Conflict) {
        return "emery plan execute".to_string();
    }
    let unknowns: Vec<_> =
        gaps.rows.iter().filter(|r| r.status == RequirementStatus::Unknown).collect();
    if unknowns.is_empty() {
        return "emery plan gaps".to_string();
    }
    let mut parts = vec!["emery plan execute".to_string()];
    for row in unknowns {
        parts.push(format!("--waive {}/{}", row.slice, row.req));
    }
    parts.push("--reason <reason>".to_string());
    parts.join(" ")
}
