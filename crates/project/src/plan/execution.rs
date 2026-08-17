//! In-memory execution-state projection: plan entry + slice tree +
//! folded journal facts become the next step (refine/build/merge/stop).
//! Shared by `plan status` and the execute loop; nothing is persisted.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::ops::ControlFlow;
use std::path::Path;

use artifacts::leads::Lead;
use error::Error;

use super::model::{Entry, Plan, Status};
use super::proposal::Proposal;
use super::status::{LoopStep, NextActionKind, StopBody, StopReason};
use crate::build_record::BuildRecord;
use crate::config::Layout;
use crate::journal::{self, Event, EventKind, ParkReason, claim};
use crate::name::{PlanName, SliceName};
use crate::refinement::{Freshness, Live};
use crate::slice::SliceMetadata;

/// Whether the active-window journal overlay applies to the candidate
/// entry. Only an in-progress entry (projected from facts) carries an
/// active window (`plan.entry.advanced` / live claim) that scopes
/// phase-terminal events to the current plan; not-yet-advanced
/// candidates skip the overlay so stale same-name events from earlier
/// plans cannot classify.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JournalOverlay {
    Apply,
    Skip,
}

/// The projected next step for one candidate entry.
#[derive(Clone)]
pub(super) struct Resolution {
    pub action: NextActionKind,
    pub slice: Option<String>,
    pub target: Option<String>,
    pub last_completed: Option<LoopStep>,
    pub stop: Option<StopBody>,
}

impl Resolution {
    pub(super) const fn stop(reason: StopReason) -> Self {
        Self {
            action: NextActionKind::Stop,
            slice: None,
            target: None,
            last_completed: None,
            stop: Some(StopBody {
                reason,
                detail: None,
                hint: reason.hint(),
            }),
        }
    }

    /// A slice-less stop with detail (the RFC-96 D8 domain gate).
    pub(super) const fn stop_detail(reason: StopReason, detail: Option<String>) -> Self {
        Self {
            action: NextActionKind::Stop,
            slice: None,
            target: None,
            last_completed: None,
            stop: Some(StopBody {
                reason,
                detail,
                hint: reason.hint(),
            }),
        }
    }

    pub(super) const fn drained() -> Self {
        Self {
            action: NextActionKind::Drained,
            slice: None,
            target: None,
            last_completed: None,
            stop: None,
        }
    }

    /// A pending publication member holds the drain (RFC-95 D11).
    pub(super) fn materialize(target: &str) -> Self {
        Self {
            action: NextActionKind::Materialize,
            slice: None,
            target: Some(target.to_string()),
            last_completed: None,
            stop: None,
        }
    }

    fn phase(action: NextActionKind, entry: &Entry, last_completed: Option<LoopStep>) -> Self {
        Self {
            action,
            slice: Some(entry.name.to_string()),
            target: Some(entry.target.clone()),
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
            target: Some(entry.target.clone()),
            last_completed,
            stop: Some(StopBody {
                reason,
                detail,
                hint: reason.hint(),
            }),
        }
    }
}

/// Load the fact union for status projection: the plan root's
/// per-writer logs.
///
/// # Errors
///
/// Propagates journal I/O failures.
pub fn collect_events(layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    let mut events = journal::read_union(layout)?;
    events.sort_by(|left, right| {
        (left.timestamp, left.writer.as_str(), left.sequence).cmp(&(
            right.timestamp,
            right.writer.as_str(),
            right.sequence,
        ))
    });
    Ok(events)
}

/// Refuse plan-topology verbs on a bound-not-authored change home.
///
/// A handoff-bound `plan.yaml` without a `plan.reconcile.completed`
/// fact may sit over a partial decomposition tree that only
/// `plan author` re-entry may finish.
///
/// # Errors
///
/// `plan-author-incomplete` on a bound plan with no reconcile fact;
/// journal I/O failures.
pub fn ensure_authored(layout: Layout<'_>, plan: &Plan) -> Result<(), Error> {
    if plan.definition.is_none() {
        return Ok(());
    }
    let events = collect_events(layout)?;
    // A park after the latest reconcile reopens authoring: a historical
    // reconcile fact must not authorize topology verbs over the parked
    // partial tree (S26).
    let mut authored = false;
    for event in &events {
        match &event.kind {
            EventKind::PlanReconcileCompleted { plan_name, .. } if plan_name == &plan.name => {
                authored = true;
            }
            EventKind::PlanAuthorParked { .. } => authored = false,
            _ => {}
        }
    }
    if authored {
        return Ok(());
    }
    Err(Error::Diag {
        code: "plan-author-incomplete",
        detail: format!(
            "plan `{name}` is bound but not authored; re-run `emery plan author` to finish \
             decomposition",
            name = plan.name
        ),
    })
}

/// Project per-entry ladder labels from the fact union (RFC-86 D2).
///
/// `done` comes from archive / postflight-failed facts;
/// `in-progress` comes from advance / a live claim; everything else is
/// `pending`.
#[must_use]
pub fn project_ladders(plan: &Plan, events: &[Event]) -> HashMap<SliceName, Status> {
    let ownership = claim::project(events);
    let mut ladders: HashMap<SliceName, Status> =
        plan.entries.iter().map(|e| (e.name.clone(), Status::Pending)).collect();
    for event in events {
        match &event.kind {
            EventKind::PlanEntryAdvanced {
                plan_name,
                slice_name,
            } if plan_name == &plan.name => {
                if ladders.contains_key(slice_name) {
                    ladders.insert(slice_name.clone(), Status::InProgress);
                }
            }
            EventKind::SliceArchiveCreated { slice_name, .. }
                if ladders.contains_key(slice_name) =>
            {
                ladders.insert(slice_name.clone(), Status::Done);
            }
            EventKind::TargetMergeWaveCommitted { members, .. }
            | EventKind::MergeWavePostflightFailed { members, .. } => {
                for name in members {
                    if ladders.contains_key(name) {
                        ladders.insert(name.clone(), Status::Done);
                    }
                }
            }
            _ => {}
        }
    }
    for (slice, _) in ownership.iter() {
        if let Some(status) = ladders.get_mut(slice)
            && *status == Status::Pending
        {
            *status = Status::InProgress;
        }
    }
    ladders
}

/// First entry in list order whose projected ladder is `pending` and
/// whose dependencies are all projected `done`. An unknown
/// `depends_on` target is treated as not done.
#[must_use]
pub fn next_eligible<'a, S: BuildHasher>(
    plan: &'a Plan, ladders: &HashMap<SliceName, Status, S>,
) -> Option<&'a Entry> {
    plan.entries.iter().find(|entry| {
        ladders.get(&entry.name).copied() == Some(Status::Pending)
            && entry.depends_on.iter().all(|dep| ladders.get(dep).copied() == Some(Status::Done))
    })
}

/// Project one candidate entry: artifact + fact phase first, then
/// (for an in-progress entry) the folded active-window journal facts.
/// `inventory` and `live` feed the refinement-freshness recompute
/// behind the refined rung (RFC-91 D2).
pub(super) fn resolve_entry(
    plan: &Plan, entry: &Entry, layout: Layout<'_>, overlay: JournalOverlay, events: &[Event],
    inventory: &[Lead], live: &mut Live,
) -> Result<Resolution, Error> {
    let slice_dir = layout.slice_dir(entry.name.as_str());

    if is_dropped(&slice_dir, &entry.name, events)? {
        return Ok(Resolution::stop_for(StopReason::SliceDropped, None, entry, None));
    }

    let facts = match overlay {
        JournalOverlay::Apply => active_window_facts(events, &plan.name, &entry.name),
        JournalOverlay::Skip => WindowFacts::default(),
    };

    // A merge that completed without the entry's `done` stamp is a torn
    // state (the slice directory is archived on merge); durable merge
    // evidence dominates any later failure marker in the window.
    if facts.merged {
        return Ok(Resolution::stop_for(
            StopReason::MergeIncomplete,
            None,
            entry,
            Some(LoopStep::Merge),
        ));
    }

    // The refined-rung verdict. Post-build, manifest presence suffices:
    // build promotion legitimately drifts bundle artifacts through
    // `writable-artifacts[]` (and `Phase::Built` dominates anyway).
    let refined = if BuildRecord::present(&slice_dir) {
        Refined::Fresh
    } else {
        match crate::refinement::freshness_with(layout, plan, entry, inventory, live)? {
            Freshness::Fresh { .. } => Refined::Fresh,
            Freshness::Missing => Refined::Missing,
            Freshness::Stale { .. } => Refined::Stale,
        }
    };

    if matches!(refined, Refined::Missing | Refined::Stale)
        && let Some(parked) = parked_refinement(layout, events, entry.name.as_str())?
    {
        return Ok(parked.into_resolution(entry));
    }

    let phase = match overlay {
        JournalOverlay::Apply => phase_progress(&slice_dir, Some(&facts), refined),
        JournalOverlay::Skip => phase_progress(&slice_dir, None, refined),
    };

    let last_completed = match phase {
        Phase::None => None,
        Phase::Refined => Some(LoopStep::Refine),
        Phase::Built => Some(LoopStep::Build),
    };

    let awaited = match phase {
        Phase::None => NextActionKind::Refine,
        Phase::Refined => NextActionKind::Build,
        Phase::Built => NextActionKind::Merge,
    };

    // Failure overlay: stop only when the newest terminal is a failure
    // of the phase the artifacts/facts are awaiting. A failure of any
    // other phase means the operator already moved the slice past it.
    if let Some((phase, stop, reason)) = facts.newest_failure
        && phase == awaited
    {
        return Ok(Resolution::stop_for(stop, Some(reason), entry, last_completed));
    }

    Ok(Resolution::phase(awaited, entry, last_completed))
}

/// Highest completed phase projected from slice artifacts (and, when
/// provided, active-window success facts).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    None,
    Refined,
    Built,
}

/// Refined-rung verdict fed into [`phase_progress`]: the manifest's
/// freshness pre-build, [`Refined::Fresh`] once a build record exists.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Refined {
    Fresh,
    Missing,
    Stale,
}

fn phase_progress(slice_dir: &Path, window: Option<&WindowFacts>, refined: Refined) -> Phase {
    // RFC-91 D2/D8: “refined” requires a FRESH manifest — stale
    // projects the refine next-action and suppresses the window's
    // refine-success bump; “built” from build records (RFC-86 D27).
    let mut phase = if BuildRecord::present(slice_dir) {
        Phase::Built
    } else if refined == Refined::Fresh && crate::slice::refinement_present(slice_dir) {
        Phase::Refined
    } else {
        Phase::None
    };
    if let Some(facts) = window {
        if facts.refined && refined != Refined::Stale {
            phase = phase_max(phase, Phase::Refined);
        }
        if facts.built {
            phase = phase_max(phase, Phase::Built);
        }
    }
    phase
}

const fn phase_max(left: Phase, right: Phase) -> Phase {
    match (left, right) {
        (Phase::Built, _) | (_, Phase::Built) => Phase::Built,
        (Phase::Refined, _) | (_, Phase::Refined) => Phase::Refined,
        (Phase::None, Phase::None) => Phase::None,
    }
}

/// Dropped when a `slice.dropped` tombstone names the slice or
/// `metadata.yaml` carries `dropped_at` — the journal fact is the
/// durable authority (S7 / CC-03); the artifact stamp leaves the live
/// tree when the slice archives.
fn is_dropped(slice_dir: &Path, slice: &SliceName, events: &[Event]) -> Result<bool, Error> {
    if super::scope::dropped(slice, events) {
        return Ok(true);
    }
    let meta = SliceMetadata::load_optional(slice_dir)?;
    Ok(meta.is_some_and(|metadata| metadata.dropped_at.is_some()))
}

/// The folded journal facts for one entry's active window.
#[derive(Default)]
struct WindowFacts {
    /// Durable merge evidence (`slice.merge.succeeded` /
    /// `slice.archive.created`) exists anywhere in the window.
    merged: bool,
    /// A refine-success terminal exists in the window.
    refined: bool,
    /// A build-success terminal exists in the window.
    built: bool,
    /// The newest phase terminal is a failure of this phase; `None`
    /// when the newest terminal is a success (or none exists).
    newest_failure: Option<(NextActionKind, StopReason, String)>,
}

/// Backward-fold `events` (newest-first walk over a chronologically
/// ordered union) over this entry's active window: events newer than
/// the entry's most recent `plan.entry.advanced`.
fn active_window_facts(events: &[Event], plan_name: &PlanName, slice: &SliceName) -> WindowFacts {
    let mut facts = WindowFacts::default();
    let mut terminal_seen = false;
    for event in events.iter().rev() {
        match &event.kind {
            EventKind::PlanEntryAdvanced {
                plan_name: p,
                slice_name: s,
            } if p == plan_name && s == slice => break,
            EventKind::SliceClaimed { slice_name: s } if s == slice => break,
            EventKind::SliceMergeSucceeded { slice_name: s } if s == slice => {
                facts.merged = true;
            }
            EventKind::SliceArchiveCreated { slice_name: s, .. } if s == slice => {
                facts.merged = true;
            }
            EventKind::TargetMergeWaveCommitted { members, .. }
                if members.iter().any(|m| m == slice) =>
            {
                facts.merged = true;
            }
            EventKind::SliceSynthesizeCompleted { slice_name: s, .. }
            | EventKind::SliceTransitionRefined { slice_name: s }
                if s == slice =>
            {
                facts.refined = true;
                terminal_seen = true;
            }
            EventKind::SliceBuildSucceeded { slice_name: s } if s == slice => {
                facts.built = true;
                terminal_seen = true;
            }
            EventKind::SliceSynthesizeFailed {
                slice_name: s,
                reason,
            } if s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Refine, StopReason::RefineFailed, reason.clone()));
                }
                terminal_seen = true;
            }
            EventKind::SliceBuildFailed {
                slice_name: s,
                reason,
            } if s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Build, StopReason::BuildFailed, reason.clone()));
                }
                terminal_seen = true;
            }
            EventKind::SliceMergeFailed {
                slice_name: s,
                reason,
            } if s == slice => {
                if !terminal_seen {
                    facts.newest_failure =
                        Some((NextActionKind::Merge, StopReason::MergeConflict, reason.clone()));
                }
                terminal_seen = true;
            }
            EventKind::MergeWavePostflightFailed { members, reason, .. }
                if members.iter().any(|m| m == slice) =>
            {
                if !terminal_seen {
                    facts.newest_failure = Some((
                        NextActionKind::Merge,
                        StopReason::MergePostflightFailed,
                        reason.clone(),
                    ));
                }
                terminal_seen = true;
            }
            _ => {}
        }
    }
    facts
}

/// An unrefined leaf parked by boundary escalation or budget exhaustion.
pub(super) struct Parked {
    reason: StopReason,
    detail: Option<String>,
}

impl Parked {
    fn into_resolution(self, entry: &Entry) -> Resolution {
        Resolution::stop_for(self.reason, self.detail, entry, None)
    }
}

pub(super) fn parked_refinement(
    layout: Layout<'_>, events: &[Event], slice: &str,
) -> Result<Option<Parked>, Error> {
    if let Some((digest, _)) = Proposal::boundary_for(layout, slice)? {
        return Ok(Some(Parked {
            reason: StopReason::BoundaryEscalation,
            detail: Some(digest.to_string()),
        }));
    }
    let mut parked = None;
    for event in events.iter().rev() {
        match &event.kind {
            EventKind::SliceRefinementParked {
                slice_name,
                reason: ParkReason::BudgetExhausted,
                ..
            } if slice_name.as_str() == slice => {
                parked = Some(Parked {
                    reason: StopReason::RefineBudgetExhausted,
                    detail: None,
                });
                break;
            }
            EventKind::SliceSynthesizeCompleted { slice_name, .. }
            | EventKind::SliceTransitionRefined { slice_name }
                if slice_name.as_str() == slice =>
            {
                break;
            }
            _ => {}
        }
    }
    Ok(parked)
}

/// Scan the union newest-first until `select` breaks — shared by the
/// sticky postflight-debt projection in `status::project`.
pub(super) fn scan_union(events: &[Event], mut select: impl FnMut(&Event) -> ControlFlow<()>) {
    for event in events.iter().rev() {
        if select(event).is_break() {
            break;
        }
    }
}
