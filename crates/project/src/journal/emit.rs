//! Best-effort journal emit and the lifecycle bracket helpers.

use jiff::Timestamp;

use super::append::{append_one, record_dropped};
use super::{Event, EventKind};
use crate::config::Layout;

/// Best-effort append of a single lifecycle [`Event`] carrying `kind`.
///
/// Stamped with the dispatcher-injected `now`; library code never
/// reads the clock. The journal is observability, not the source of
/// truth, so a failed append is intentionally swallowed — it can never
/// change the calling verb's exit code. The swallow is not silent:
/// `record_dropped` warns on stderr and appends the dropped event to
/// the `.emery/journal.dropped` sidecar.
pub fn emit_best_effort(layout: Layout<'_>, now: Timestamp, kind: EventKind, scope: &str) {
    let event = Event::new(now, kind);
    if let Err(err) = append_one(layout, &event) {
        record_dropped(layout, scope, &event, &err);
    }
}

/// Best-effort lifecycle bracket around one fallible async phase body:
/// emit `started`, await `body`, then emit `on_success(&ok)` or
/// `on_failure(&err)` and pass the result through unchanged.
///
/// Every emit goes through [`emit_best_effort`]; strict journal writes
/// (the claim event, synthesis tags) stay `append_one` / `append_batch`
/// at their call sites.
///
/// # Errors
///
/// Whatever `body` returns, unchanged.
pub async fn bracket<T, Fut>(
    layout: Layout<'_>, now: Timestamp, scope: &str, started: EventKind, body: Fut,
    on_success: impl FnOnce(&T) -> EventKind, on_failure: impl FnOnce(&error::Error) -> EventKind,
) -> Result<T, error::Error>
where
    Fut: Future<Output = Result<T, error::Error>>,
{
    emit_best_effort(layout, now, started, scope);
    settle(layout, now, scope, body.await, on_success, on_failure)
}

/// Shared terminal emit for the bracket.
fn settle<T>(
    layout: Layout<'_>, now: Timestamp, scope: &str, result: Result<T, error::Error>,
    on_success: impl FnOnce(&T) -> EventKind, on_failure: impl FnOnce(&error::Error) -> EventKind,
) -> Result<T, error::Error> {
    match result {
        Ok(value) => {
            emit_best_effort(layout, now, on_success(&value), scope);
            Ok(value)
        }
        Err(err) => {
            emit_best_effort(layout, now, on_failure(&err), scope);
            Err(err)
        }
    }
}
