//! Best-effort journal emit.

use jiff::Timestamp;

use super::append::{append_batch, record_dropped};
use super::{Event, EventKind};
use crate::config::Layout;

/// Best-effort append of a single lifecycle [`Event`] carrying `kind`.
///
/// Stamped with the dispatcher-injected `now` (architecture.md §"Time
/// injection"); library code never reads the clock. The journal is
/// observability, not the
/// source of truth, so a failed append is **intentionally swallowed** —
/// it can never change the calling verb's exit code (a journaling I/O
/// hiccup must not fail an otherwise-successful slice merge / build). The
/// lifecycle brackets in `slice merge` / `slice build` emit through this.
///
/// The swallow is intentional but **not silent**: `record_dropped`
/// routes a structured `warning:` line to stderr (naming `scope`, the
/// journal path, and the I/O error) through the same operator-warning
/// surface other best-effort failures use, and appends the dropped event
/// to the `<project_dir>/.specify/journal.dropped` sidecar as a
/// recoverable audit trail. The mitigation is itself best-effort and
/// never panics.
pub fn emit_best_effort(layout: Layout<'_>, now: Timestamp, kind: EventKind, scope: &str) {
    let event = Event::new(now, kind);
    if let Err(err) = append_batch(layout, std::slice::from_ref(&event)) {
        record_dropped(layout, scope, &event, &err);
    }
}
