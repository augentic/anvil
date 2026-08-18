//! Shared plan membership predicate.
//!
//! **In-scope** means the entry is on the plan and its slice is not
//! dropped; gaps, Ready, the execute gate, and status must share this filter.

use super::model::{Entry, Plan};
use crate::journal::{Event, EventKind};
use crate::slice::SliceMetadata;

/// Whether `entry` is in-scope for gaps / Ready / execute / unrefined
/// next-actions (RFC-86 D24).
///
/// True when `entry` is on `plan`, no `slice.dropped` fact names it,
/// and `meta` has no `dropped_at` stamp. The journal tombstone is the
/// durable scope authority (S7 / CC-03): archiving moves the stamped
/// `metadata.yaml` out of the live tree, so a dropped entry stays
/// excluded with no live metadata — while a missing document without
/// a tombstone (slice not yet created) stays in-scope.
#[must_use]
pub fn in_scope(
    plan: &Plan, entry: &Entry, meta: Option<&SliceMetadata>, events: &[Event],
) -> bool {
    !dropped(&entry.name, events)
        && plan.entries.iter().any(|e| e.name == entry.name)
        && meta.is_none_or(|m| m.dropped_at.is_none())
}

/// Whether a `slice.dropped` tombstone names `slice`.
#[must_use]
pub fn dropped(slice: &crate::name::SliceName, events: &[Event]) -> bool {
    events.iter().any(|event| {
        matches!(&event.kind, EventKind::SliceDropped { slice_name, .. } if slice_name == slice)
    })
}
