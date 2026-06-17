//! Best-effort journal emit and the `lint-completed` projection.

use jiff::Timestamp;
use specify_diagnostics::{Diagnostic, FindingStatus, count_status};

use super::append::{append_batch, record_dropped};
use super::{Event, EventKind, LintCompletedPayload, LintCounts, LintScope};
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

/// Append a `lint-completed` event to `<project_dir>/.specify/journal.jsonl`.
///
/// Best-effort: a serialise/IO failure is intentionally swallowed so it
/// never overrides the scan's exit code. The swallow is not silent —
/// `record_dropped` warns on stderr under `command_label` and records
/// the dropped event in the `.specify/journal.dropped` sidecar.
pub fn emit_lint_completed(
    layout: Layout<'_>, now: Timestamp, scope: LintScope, findings: &[Diagnostic],
    duration_ms: u128, exit_code: i32, command_label: &str,
) {
    let counts = LintCounts {
        open: count_status(findings, None),
        ignored: count_status(findings, Some(FindingStatus::Ignored)),
        false_positive: count_status(findings, Some(FindingStatus::FalsePositive)),
    };
    let payload = LintCompletedPayload {
        scope,
        duration_ms: u64::try_from(duration_ms).unwrap_or(u64::MAX),
        counts,
        exit_code,
    };
    let event = Event::new(now, EventKind::LintCompleted(payload));
    if let Err(err) = append_batch(layout, std::slice::from_ref(&event)) {
        record_dropped(layout, command_label, &event, &err);
    }
}
