//! The in-memory execution-state projection: one candidate plan
//! entry, its work-root slice tree, and the folded active-window
//! journal facts become the next-step decision (refine / build /
//! merge / stop).
//!
//! Shared by the read-only `plan status` projection and (through it)
//! the guest execute loop, so the two surfaces cannot drift. Nothing
//! here is persisted — the projection folds durable state (the plan
//! topology, slice artifacts, journal events) on every call.
//!
//! **Authority (RFC-86 D2):** progress is computed from artifacts and
//! the fact union. No stored entry or slice status fields are read
//! are not read. Ladder labels (`pending` / `in-progress` / `done`)
//! project from claim / advance / undo / archive facts; the awaited
//! phase projects from slice artifacts plus refine/build success
//! facts.
//!
//! Journal failure / durable-merge overlays fold over the entry's
//! **active window**: events newer than the entry's most recent
//! `plan.entry.advanced` / `plan.transition.undone`. Within the
//! window, durable merge evidence (`slice.merge.succeeded` /
//! `slice.archive.created`) dominates any later failure marker — a
//! failed retry after a landed merge is noise, and the torn state
//! projects the `merge-incomplete` stop. Otherwise the newest phase
//! terminal decides: a success means "dispatch on phase artifacts",
//! a failure of the awaited phase parks the matching stop.

use std::collections::{BTreeSet, HashMap};
use std::hash::BuildHasher;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use error::Error;

use super::model::{Entry, Plan, Status};
use super::status::{LoopStep, NextActionKind, StopBody, StopReason};
use crate::build_record::BuildRecord;
use crate::config::Layout;
use crate::journal::{self, Event, EventKind, claim};
use crate::name::{PlanName, SliceName};
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

/// Load the fact union for status projection: the plan root's
/// per-actor logs plus each distinct materialised workspace-slot
/// journal (phase work for a project-bound entry writes there).
///
/// # Errors
///
/// Propagates journal I/O failures from any root.
pub fn collect_events(plan: &Plan, layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    let mut roots = BTreeSet::new();
    roots.insert(layout.project_dir().to_path_buf());
    for entry in &plan.entries {
        roots.insert(resolve_work_root(layout, entry));
    }
    let mut events = Vec::new();
    for root in &roots {
        events.extend(journal::read_union(Layout::new(root))?);
    }
    events.sort_by(|left, right| {
        (left.timestamp, left.actor.as_str(), left.sequence).cmp(&(
            right.timestamp,
            right.actor.as_str(),
            right.sequence,
        ))
    });
    Ok(events)
}

/// Project per-entry ladder labels from the fact union (RFC-86 D2).
///
/// `done` comes from archive /
/// postflight-failed facts (and undo walks them back by retracting);
/// `in-progress` comes from advance / a live claim; everything else is
/// `pending`. Retracted facts (`fact.retracted`) are omitted.
#[must_use]
pub fn project_ladders(plan: &Plan, events: &[Event]) -> HashMap<SliceName, Status> {
    let ownership = claim::project(events);
    let retracted = claim::retracted_targets(events);
    let mut ladders: HashMap<SliceName, Status> =
        plan.entries.iter().map(|e| (e.name.clone(), Status::Pending)).collect();
    for event in events {
        if retracted.contains(&(event.actor.as_str(), event.sequence)) {
            continue;
        }
        match &event.kind {
            EventKind::PlanEntryAdvanced {
                plan_name,
                slice_name,
            } if plan_name == &plan.name => {
                if ladders.contains_key(slice_name) {
                    ladders.insert(slice_name.clone(), Status::InProgress);
                }
            }
            EventKind::PlanTransitionUndone {
                plan_name,
                slice_name,
                to,
                ..
            } if plan_name == &plan.name => {
                if ladders.contains_key(slice_name) {
                    ladders.insert(slice_name.clone(), *to);
                }
            }
            EventKind::SliceArchiveCreated { slice_name, .. }
            | EventKind::TargetMergeWaveCommitted { slice_name, .. }
            | EventKind::TargetMergeWavePostflightFailed { slice_name, .. }
                if ladders.contains_key(slice_name) =>
            {
                ladders.insert(slice_name.clone(), Status::Done);
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
pub(super) fn resolve_entry(
    plan: &Plan, entry: &Entry, layout: Layout<'_>, overlay: JournalOverlay, events: &[Event],
) -> Result<Resolution, Error> {
    let work_root = resolve_work_root(layout, entry);
    let work_layout = Layout::new(&work_root);
    let slice_dir = work_layout.slice_dir(entry.name.as_str());

    if is_dropped(&slice_dir)? {
        return Ok(Resolution::stop_for(StopReason::SliceDropped, None, entry, None));
    }

    let facts = match overlay {
        JournalOverlay::Apply => active_window_facts(events, &plan.name, &entry.name),
        JournalOverlay::Skip => WindowFacts::default(),
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

    let phase = match overlay {
        JournalOverlay::Apply => phase_progress(&slice_dir, Some(&facts)),
        JournalOverlay::Skip => phase_progress(&slice_dir, None),
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

/// Highest completed phase projected from slice artifacts (and, when
/// provided, active-window success facts).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    None,
    Refined,
    Built,
}

fn phase_progress(slice_dir: &Path, window: Option<&WindowFacts>) -> Phase {
    // RFC-86 D27: “built” from fact-substrate build records, not
    // `build/patch.yaml`.
    let mut phase = if BuildRecord::present(slice_dir) {
        Phase::Built
    } else if slice_dir.join("model.yaml").is_file() || slice_dir.join("spec.md").is_file() {
        Phase::Refined
    } else {
        Phase::None
    };
    if let Some(facts) = window {
        if facts.refined {
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

/// Dropped when `metadata.yaml` carries `dropped_at` — the abandon
/// stamp is an artifact field, not the lifecycle status enum (RFC-86
/// D2 / D24).
fn is_dropped(slice_dir: &Path) -> Result<bool, Error> {
    match SliceMetadata::load(slice_dir) {
        Ok(metadata) => Ok(metadata.dropped_at.is_some()),
        Err(
            Error::ArtifactNotFound { .. }
            | Error::Diag {
                code: "slice-not-found",
                ..
            },
        ) => Ok(false),
        Err(err) => Err(err),
    }
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
/// ordered union) over this entry's active window.
fn active_window_facts(events: &[Event], plan_name: &PlanName, slice: &SliceName) -> WindowFacts {
    let retracted = claim::retracted_targets(events);
    let mut facts = WindowFacts::default();
    let mut terminal_seen = false;
    for event in events.iter().rev() {
        if retracted.contains(&(event.actor.as_str(), event.sequence)) {
            continue;
        }
        match &event.kind {
            EventKind::PlanEntryAdvanced {
                plan_name: p,
                slice_name: s,
            }
            | EventKind::PlanTransitionUndone {
                plan_name: p,
                slice_name: s,
                ..
            } if p == plan_name && s == slice => break,
            EventKind::SliceClaimed { slice_name: s } if s == slice => break,
            EventKind::SliceMergeSucceeded { slice_name: s } if s == slice => {
                facts.merged = true;
            }
            EventKind::SliceArchiveCreated { slice_name: s, .. }
            | EventKind::TargetMergeWaveCommitted { slice_name: s, .. }
                if s == slice =>
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
            EventKind::TargetMergeWavePostflightFailed {
                slice_name: s,
                reason,
                ..
            } if s == slice => {
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

/// Scan the union newest-first until `select` breaks — shared by the
/// sticky postflight-debt projection in `status::project`.
pub(super) fn scan_union(events: &[Event], mut select: impl FnMut(&Event) -> ControlFlow<()>) {
    for event in events.iter().rev() {
        if select(event).is_break() {
            break;
        }
    }
}
