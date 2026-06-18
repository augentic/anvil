//! Read-only `specify plan status` projection.
//!
//! [`plan_status_body`] projects `plan.yaml` entries, the candidate
//! slice's `metadata.yaml` lifecycle, and the journal tail into a
//! deterministic `next-action` — `refine|build|merge <slice>`,
//! `stop <reason>`, or `drained` — so `/spec:execute` renders the
//! dispatch instead of deriving it. Writes nothing; `plan next`
//! stays the only writer of per-entry `in-progress`.

use std::path::PathBuf;

use serde::Serialize;
use specify_error::Error;

use super::model::{Entry, Lifecycle, Plan, Status};
use crate::config::Layout;
use crate::journal::{self, EventKind};
use crate::name::{PlanName, SliceName};
use crate::slice::{LifecycleStatus, SliceMetadata};

/// Closed next-action verb set on [`StatusBody::action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum NextActionKind {
    /// Run `/spec:refine` for [`StatusBody::slice`].
    Refine,
    /// Run `/spec:build` for [`StatusBody::slice`].
    Build,
    /// Run `/spec:merge` for [`StatusBody::slice`].
    Merge,
    /// Halt the loop; [`StatusBody::stop`] carries the reason.
    Stop,
    /// No pending or in-progress entries remain — the only clean exit.
    Drained,
}

/// Closed slice-loop step set for the RM-15 re-entry fields
/// ([`StatusBody::current_step`] / [`StatusBody::last_completed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LoopStep {
    /// The refine phase (`/spec:refine`).
    Refine,
    /// The build phase (`/spec:build`).
    Build,
    /// The merge phase (`/spec:merge`, including the per-entry `done` stamp).
    Merge,
}

/// Closed stop-reason set on [`StopBody::reason`].
///
/// The three loop stops (`refine-failed` / `build-failed` /
/// `merge-conflict`) carry the stop-conditions reference's structured
/// strings; the rest are pre-loop or repair conditions the driver
/// renders the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StopReason {
    /// `plan.lifecycle` is still `pending` — Gate 1 has not stamped.
    PlanNotApproved,
    /// The awaited refine phase last ended in `slice.synthesize.failed`.
    RefineFailed,
    /// The awaited build phase last ended in `slice.build.failed`.
    BuildFailed,
    /// The awaited merge phase last ended in `slice.merge.failed`.
    MergeConflict,
    /// The active entry's slice was dropped without merging.
    SliceDropped,
    /// The slice merged but the entry is still `in-progress` — the
    /// `done` stamp is missing.
    MergeIncomplete,
    /// Pending entries remain but every one waits on unmet dependencies.
    Stuck,
}

impl StopReason {
    /// Operator hint rendered under the stop block — one line, aligned
    /// with the stop-conditions reference's re-entry contract.
    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::PlanNotApproved => {
                "Stamp Gate 1 first: specify plan transition <plan-name> approved."
            }
            Self::RefineFailed => {
                "Fix the failure, then retry /spec:refine for the slice. The plan entry stays \
                 in-progress."
            }
            Self::BuildFailed => {
                "Fix the failure, then retry /spec:build for the slice. The plan entry stays \
                 in-progress."
            }
            Self::MergeConflict => {
                "Resolve the baseline conflict (or drop the slice), then retry /spec:merge. The \
                 plan entry stays in-progress until the merge lands."
            }
            Self::SliceDropped => {
                "The slice was dropped; amend or remove the plan entry to unblock the queue."
            }
            Self::MergeIncomplete => {
                "The slice is merged but the entry is still in-progress; stamp it with specify \
                 plan transition <entry> done."
            }
            Self::Stuck => {
                "Remaining entries wait on unmet dependencies; complete or amend the blocking \
                 entries."
            }
        }
    }
}

/// Stop sub-body on [`StatusBody::stop`], populated when
/// [`StatusBody::action`] is [`NextActionKind::Stop`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StopBody {
    /// Why the loop must halt.
    pub reason: StopReason,
    /// Failure detail from the journal event payload, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// One-line operator hint for this stop.
    pub hint: &'static str,
}

/// Per-status entry counts on [`StatusBody::counts`].
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusCounts {
    /// Entries at `pending`.
    pub pending: usize,
    /// Entries at `in-progress`.
    pub in_progress: usize,
    /// Entries at `done`.
    pub done: usize,
}

/// Wire body for `specify plan status` (text + JSON).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusBody {
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Plan-level lifecycle (`pending | approved`).
    pub lifecycle: Lifecycle,
    /// Per-status entry counts.
    pub counts: StatusCounts,
    /// Name of the active `in-progress` entry, when one exists.
    pub active: Option<String>,
    /// Rendered projection — `refine|build|merge <slice>`,
    /// `stop <reason>`, or `drained`.
    pub next_action: String,
    /// Closed verb behind [`Self::next_action`].
    pub action: NextActionKind,
    /// Slice the action targets; `None` on `stop`-without-slice and
    /// `drained`.
    pub slice: Option<String>,
    /// Bound project of the targeted entry, when set.
    pub project: Option<String>,
    /// RM-15: step the targeted slice is currently at — the awaited
    /// phase, including a phase the loop is stopped on. `None` when no
    /// slice is targeted (pre-Gate-1, `stuck`, `slice-dropped`,
    /// `drained`).
    pub current_step: Option<LoopStep>,
    /// RM-15: most recent step the targeted slice completed, from its
    /// lifecycle (`refined` → `refine`, `built` → `build`, a landed
    /// merge → `merge`). `None` before the first phase completes or
    /// when no slice is targeted.
    pub last_completed: Option<LoopStep>,
    /// RM-15: next valid resume point as a literal command — the phase
    /// skill for dispatches and retryable stops, the Gate 1 / `done`
    /// stamp for the stamp-shaped stops, `/spec:finalize` on drained.
    /// `None` when no single command makes progress (`stuck`,
    /// `slice-dropped`).
    pub resume: Option<String>,
    /// Stop classification, populated when [`Self::action`] is
    /// [`NextActionKind::Stop`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopBody>,
}

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

/// RM-15 `current-step`: the phase the targeted slice is at — the
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

/// RM-15 `resume`: the next valid resume point as a literal command.
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

    // RM-15 `last-completed`: the slice lifecycle is the record of the
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
        let slot = layout.plan_dir().join("workspace").join(project);
        if slot.is_dir() {
            return slot;
        }
    }
    layout.project_dir().to_path_buf()
}

/// Newest journal marker relevant to the active entry's claim.
enum Marker {
    /// `plan.entry.advanced` / `plan.transition.undone` for this
    /// `(plan, slice)`, or a phase success — all mean "dispatch on
    /// lifecycle".
    Neutral,
    /// `slice.merge.succeeded` / `slice.archive.created` — the merge
    /// landed; only the entry stamp can be missing.
    MergeSucceeded,
    /// The newest terminal event is a phase failure.
    PhaseFailed { phase: NextActionKind, reason: String },
}

/// Backward-scan the work root's journal for the newest event that
/// marks this entry's claim window or a phase terminal for its slice.
fn newest_marker(
    work_layout: Layout<'_>, plan_name: &PlanName, slice: &SliceName,
) -> Result<Option<Marker>, Error> {
    let mut found = journal::read_recent(work_layout, 1, |event| match event.kind {
        EventKind::PlanEntryAdvanced {
            plan_name: p,
            slice_name: s,
        }
        | EventKind::PlanTransitionUndone {
            plan_name: p,
            slice_name: s,
            ..
        } if &p == plan_name && &s == slice => Some(Marker::Neutral),
        EventKind::SliceSynthesizeCompleted { slice_name: s, .. }
        | EventKind::SliceBuildSucceeded { slice_name: s }
            if &s == slice =>
        {
            Some(Marker::Neutral)
        }
        EventKind::SliceMergeSucceeded { slice_name: s } if &s == slice => {
            Some(Marker::MergeSucceeded)
        }
        EventKind::SliceArchiveCreated { slice_name: s, .. } if &s == slice => {
            Some(Marker::MergeSucceeded)
        }
        EventKind::SliceSynthesizeFailed {
            slice_name: s,
            reason,
        } if &s == slice => Some(Marker::PhaseFailed {
            phase: NextActionKind::Refine,
            reason,
        }),
        EventKind::SliceBuildFailed {
            slice_name: s,
            reason,
        } if &s == slice => Some(Marker::PhaseFailed {
            phase: NextActionKind::Build,
            reason,
        }),
        EventKind::SliceMergeFailed {
            slice_name: s,
            reason,
        } if &s == slice => Some(Marker::PhaseFailed {
            phase: NextActionKind::Merge,
            reason,
        }),
        _ => None,
    })?;
    Ok(found.pop())
}

/// Stop-conditions drained string: `drained — run /spec:finalize <name>`.
#[must_use]
pub fn drained_line(plan_name: &str) -> String {
    format!("drained \u{2014} run /spec:finalize {plan_name}")
}

#[cfg(test)]
mod tests;
