//! Journal classification for the status projection: the newest event
//! relevant to the active entry's claim window.

use error::Error;

use super::NextActionKind;
use crate::config::Layout;
use crate::journal::{self, EventKind};
use crate::name::{PlanName, SliceName};

/// Newest journal marker relevant to the active entry's claim.
pub(super) enum Marker {
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
pub(super) fn newest_marker(
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
