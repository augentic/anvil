//! The status projection: plan topology + artifacts + the fact union
//! → one [`StatusBody`] (RFC-86 D2 / D22 / D26 / RFC-86a / RFC-91).

use std::ops::ControlFlow;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;

use super::super::execution::{
    JournalOverlay, Resolution, collect_events, next_eligible, project_ladders, resolve_entry,
    scan_union,
};
use super::super::gaps::{GapsBody, plan_gaps_body};
use super::super::model::{Entry, Plan, Status};
use super::super::{in_scope, publication};
use super::{LoopStep, NextActionKind, StatusBody, StatusCounts, StopReason};
use crate::build_record::BuildRecord;
use crate::config::Layout;
use crate::journal::{Event, EventKind};
use crate::name::SliceName;
use crate::refinement::{Freshness, Live};
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
    // In-progress entries in the canonical work order (RFC-96): the
    // head carries the singular fields; every row lands on
    // `in-progress[]`.
    let mut active_entries: Vec<(usize, &Entry)> = plan
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| ladders.get(&e.name).copied() == Some(Status::InProgress))
        .collect();
    let layer_map = super::super::schedule::layers(plan);
    active_entries.sort_by_key(|(index, entry)| {
        (
            entry.target.clone(),
            layer_map.get(&entry.name).copied().unwrap_or_default(),
            *index,
            entry.name.to_string(),
        )
    });
    let active = active_entries.first().map(|(_, entry)| *entry);

    // One freshness cache and lead inventory serve the resolution and
    // the Ready milestone; an absent `leads.md` degrades to an
    // empty inventory (planning drift then reads as staleness).
    let mut live = Live::new();
    let inventory = load_inventory(layout)?;

    let mut in_progress = Vec::with_capacity(active_entries.len());
    let mut head_resolution = None;
    for (_, entry) in &active_entries {
        let resolution = resolve_entry(
            plan,
            entry,
            layout,
            JournalOverlay::Apply,
            &events,
            &inventory,
            &mut live,
        )?;
        in_progress.push(super::InProgressBody {
            slice: entry.name.to_string(),
            target: entry.target.clone(),
            phase: current_step(&resolution),
            stop: resolution.stop.clone(),
        });
        if head_resolution.is_none() {
            head_resolution = Some(resolution);
        }
    }

    let resolution = match head_resolution {
        Some(resolution) => resolution,
        None => {
            // Sticky postflight debt: after a non-rollback postflight
            // failure the entry is already `done`, so nothing is
            // in-progress — project the stop until execute acknowledges.
            if let Some(debt) = postflight_debt(plan, &events) {
                debt
            } else {
                match next_eligible(plan, &ladders) {
                    Some(entry) => resolve_entry(
                        plan,
                        entry,
                        layout,
                        JournalOverlay::Skip,
                        &events,
                        &inventory,
                        &mut live,
                    )?,
                    None if ladders.values().all(|s| *s == Status::Done) => Resolution::drained(),
                    None => Resolution::stop(StopReason::Stuck),
                }
            }
        }
    };
    let gaps = plan_gaps_body(plan, layout, &events)?;
    let milestones = Milestones {
        ready: all_in_scope_refined(plan, layout, &inventory, &mut live)? && clean_gaps(&gaps),
        authorized: project_authorized(plan, layout, &events)?,
    };
    // The drain condition includes publication (RFC-95 D11): a plan
    // whose entries are all done but whose members lack materialized
    // facts projects `materialize`, not `drained`.
    let members = publication::members(plan, layout, &events)?;
    let resolution = match resolution {
        r if r.action == NextActionKind::Drained => members
            .iter()
            .find(|member| member.pending())
            .map_or(r, |member| Resolution::materialize(&member.target)),
        r => r,
    };
    let publication = publication_bodies(plan, &members);
    Ok(assemble(
        plan,
        counts,
        active,
        &ladders,
        resolution,
        gaps,
        milestones,
        publication,
        in_progress,
    ))
}

/// Per-member publication milestone rows, in member order.
fn publication_bodies(
    plan: &Plan, members: &[publication::Member],
) -> Vec<super::PublicationMemberBody> {
    use super::{PublicationMemberBody, PublicationMemberState};
    members
        .iter()
        .map(|member| {
            let (state, next) = match &member.materialized {
                Some(fact) => (
                    PublicationMemberState::Materialized,
                    format!(
                        "review, commit, and push branch {} from {}",
                        fact.branch, fact.worktree_path
                    ),
                ),
                None if member.blocked => (
                    PublicationMemberState::Blocked,
                    "acknowledge the postflight stop by re-running emery plan execute".to_string(),
                ),
                None if member.complete && member.accepted.is_some() => (
                    PublicationMemberState::Ready,
                    "emery plan execute materializes the publication worktree".to_string(),
                ),
                None => (
                    PublicationMemberState::AwaitingMerges,
                    format!("awaiting in-scope merges for plan {}", plan.name),
                ),
            };
            PublicationMemberBody {
                target: member.target.clone(),
                state,
                branch: member.materialized.as_ref().map(|fact| fact.branch.clone()),
                worktree: member.materialized.as_ref().map(|fact| fact.worktree_path.clone()),
                next,
            }
        })
        .collect()
}

/// Plan-wide Ready / Authorized inputs for [`assemble`] (RFC-86 D22).
#[derive(Clone, Copy)]
struct Milestones {
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
        EventKind::MergeWavePostflightFailed { members, reason, .. } => plan
            .entries
            .iter()
            .find(|e| members.contains(&e.name))
            .map_or(ControlFlow::Continue(()), |entry| {
                resolution = Some(Resolution::stop_for(
                    StopReason::MergePostflightFailed,
                    Some(reason.clone()),
                    entry,
                    Some(LoopStep::Merge),
                ));
                ControlFlow::Break(())
            }),
        EventKind::PostflightAcknowledged { slice_name }
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

/// Ready: zero open **and** zero deferred findings (unknowns /
/// conflicts). Dispositions never contribute to Ready, so a
/// debt-carrying plan reaches build via Authorized only (D22 /
/// RFC-86a D7). Divergence is listed but does not block Ready.
fn clean_gaps(gaps: &GapsBody) -> bool {
    !gaps
        .rows
        .iter()
        .any(|row| matches!(row.status, RequirementStatus::Unknown | RequirementStatus::Conflict))
}

/// Every in-scope entry counts as refined (RFC-91 D2): a FRESH
/// refinement manifest, or — once a build record exists — manifest
/// presence, because build promotion legitimately drifts bundle
/// artifacts through `writable-artifacts[]` (the same carve-out
/// execute's coverage assembly applies). Empty in-scope set is
/// vacuously refined.
fn all_in_scope_refined(
    plan: &Plan, layout: Layout<'_>, inventory: &[artifacts::leads::Lead], live: &mut Live,
) -> Result<bool, Error> {
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if BuildRecord::present(&slice_dir) {
            if crate::slice::refinement_present(&slice_dir) {
                continue;
            }
            return Ok(false);
        }
        match crate::refinement::freshness_with(layout, plan, entry, inventory, live)? {
            Freshness::Fresh { .. } => {}
            Freshness::Missing | Freshness::Stale { .. } => return Ok(false),
        }
    }
    Ok(true)
}

/// The full `leads.md` catalog; an absent file degrades to
/// an empty set the way the freshness callers tolerate today.
fn load_inventory(layout: Layout<'_>) -> Result<Vec<artifacts::leads::Lead>, Error> {
    let path = layout.leads_path();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    Ok(artifacts::leads::Leads::load(&path)?.leads().to_vec())
}

/// Authorized when the newest `plan.execute.started` epoch still
/// covers the live plan / refinement digests — the same freshness the
/// execute gap gate enforces before build (RFC-86 D22 / RFC-91 D5).
fn project_authorized(plan: &Plan, layout: Layout<'_>, events: &[Event]) -> Result<bool, Error> {
    let freshness = super::super::epoch::freshness(layout, plan, events)?;
    Ok(matches!(freshness, super::super::epoch::EpochFreshness::Fresh { .. }))
}

#[expect(clippy::too_many_arguments, reason = "one-shot assembly of the wire body's inputs")]
fn assemble(
    plan: &Plan, counts: StatusCounts, active: Option<&Entry>,
    ladders: &std::collections::HashMap<SliceName, Status>, resolution: Resolution, gaps: GapsBody,
    milestones: Milestones, publication: Vec<super::PublicationMemberBody>,
    in_progress: Vec<super::InProgressBody>,
) -> StatusBody {
    let next_action = match (resolution.action, &resolution.slice, &resolution.stop) {
        (NextActionKind::Drained, ..) => "drained".to_string(),
        (NextActionKind::Stop, _, Some(stop)) => format!("stop {}", stop.reason),
        // Materialize targets a publication member, not a slice.
        (NextActionKind::Materialize, ..) => {
            format!("materialize {}", resolution.target.as_deref().unwrap_or_default())
        }
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
        ready: milestones.ready,
        authorized: milestones.authorized,
        debt: gaps.debt(),
        slice: resolution.slice,
        target: resolution.target,
        stop: resolution.stop,
        publication,
        gaps,
        in_progress,
    }
}

/// `current-step`: the phase the targeted slice is at — the
/// dispatched phase, or the phase a stop is parked on.
fn current_step(resolution: &Resolution) -> Option<LoopStep> {
    match resolution.action {
        NextActionKind::Refine => Some(LoopStep::Refine),
        NextActionKind::Build => Some(LoopStep::Build),
        NextActionKind::Merge => Some(LoopStep::Merge),
        NextActionKind::Materialize | NextActionKind::Drained => None,
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed
            | StopReason::RefinementRequired
            | StopReason::BoundaryEscalation
            | StopReason::RefineBudgetExhausted => Some(LoopStep::Refine),
            StopReason::BuildFailed => Some(LoopStep::Build),
            // `merge-incomplete` parks inside merge: the spec merge landed
            // but the per-entry `done` stamp has not. Postflight failure is
            // past merge (`done` + archived) — no awaited phase.
            StopReason::MergeConflict | StopReason::MergeIncomplete => Some(LoopStep::Merge),
            StopReason::MergePostflightFailed
            | StopReason::SliceDropped
            | StopReason::Stuck
            | StopReason::PublicationWorktreeDirty
            | StopReason::PublicationProvision => None,
        }),
    }
}

/// `resume`: the next valid resume point as a literal command.
/// `None` when no single command makes progress.
fn resume_point(
    plan: &Plan, ladders: &std::collections::HashMap<SliceName, Status>, resolution: &Resolution,
) -> Option<String> {
    // A fresh plan (no entry has left projected `pending`) resumes
    // through refine or execute per the projected action (RFC-91 D8 /
    // D26) — open gaps never redirect it: the gate defers them.
    if ladders.values().all(|s| *s == Status::Pending)
        && matches!(
            resolution.action,
            NextActionKind::Refine | NextActionKind::Build | NextActionKind::Merge
        )
    {
        return Some(if resolution.action == NextActionKind::Refine {
            "/emery:refine".to_string()
        } else {
            "/emery:execute".to_string()
        });
    }
    match resolution.action {
        // Refinement resumes through `plan refine` (RFC-91 D1/D8);
        // build, merge, and publication materialize resume through
        // the execute loop.
        NextActionKind::Refine => Some("emery plan refine".to_string()),
        NextActionKind::Build | NextActionKind::Merge | NextActionKind::Materialize => {
            Some("emery plan execute".to_string())
        }
        NextActionKind::Drained => Some(format!("/emery:finalize {}", plan.name)),
        NextActionKind::Stop => resolution.stop.as_ref().and_then(|stop| match stop.reason {
            StopReason::RefineFailed
            | StopReason::RefinementRequired
            | StopReason::RefineBudgetExhausted => Some("emery plan refine".to_string()),
            StopReason::BuildFailed
            | StopReason::MergeConflict
            | StopReason::MergePostflightFailed
            | StopReason::MergeIncomplete
            | StopReason::PublicationWorktreeDirty
            | StopReason::PublicationProvision => Some("emery plan execute".to_string()),
            StopReason::SliceDropped | StopReason::Stuck => None,
            StopReason::BoundaryEscalation => {
                stop.detail.as_ref().map(|digest| format!("emery plan amend --proposal {digest}"))
            }
        }),
    }
}
