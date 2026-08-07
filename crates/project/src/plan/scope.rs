//! Shared plan membership predicate (RFC-86 D24).
//!
//! **In-scope** means the entry is currently on the plan and its slice
//! is not dropped. Gaps projection, Ready, the execute gap gate, and
//! status's unrefined next-actions must share this filter — they must
//! not invent divergent membership rules.

use super::model::{Entry, Plan};
use crate::slice::SliceMetadata;

/// Whether `entry` is in-scope for gaps / Ready / execute / unrefined
/// next-actions (RFC-86 D24).
///
/// True when `entry` is on `plan` and `meta` has no `dropped_at`
/// stamp. A missing metadata document (slice not yet created) is not
/// dropped, so the entry stays in-scope. `plan remove` deletes the row
/// (absent ⇒ not in-scope); `slice drop` abandons the slice and
/// excludes it even when the plan row remains.
#[must_use]
pub fn in_scope(plan: &Plan, entry: &Entry, meta: Option<&SliceMetadata>) -> bool {
    plan.entries.iter().any(|e| e.name == entry.name) && meta.is_none_or(|m| m.dropped_at.is_none())
}
