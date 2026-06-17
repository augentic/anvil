//! Journal append plus the dropped-event sidecar recovery trail.

use std::io::Write;

use specify_error::Error;

use super::{Event, path};
use crate::config::Layout;

/// Project-relative path of the dropped-event sidecar. A best-effort
/// append failure (see [`super::emit_best_effort`]) gets a second,
/// recoverable home here so an `O_APPEND` hiccup to the primary
/// journal is never a silent loss.
pub(super) const DROPPED_FILE_NAME: &str = "journal.dropped";

/// Append a sequence of [`Event`]s to the project journal in a
/// single write call.
///
/// Opens `<project_dir>/.specify/journal.jsonl` in append mode,
/// creating the file (and the `.specify/` directory) on first
/// write. All events are serialised, concatenated as
/// newline-terminated JSON lines, and pushed through one
/// `write_all` followed by one `sync_all`. Either every line
/// lands on disk or none does — downstream consumers never
/// observe a partial-state batch. A POSIX `O_APPEND` write of
/// ≤ `PIPE_BUF` bytes is atomic against concurrent writers on
/// local filesystems, which is the safety envelope a workflow
/// journal needs — the workflow contract emits one event per CLI verb
/// invocation, well below the limit.
///
/// Used by CLI verbs that own more than one journal emit per
/// invocation (e.g. `specify plan create --auto-approve
/// --authority-override`, which stages both `plan.transition.approved`
/// and `plan.amend.authority-override` in the same Gate-1 consent), and
/// equally by single-event callers via
/// `append_batch(layout, std::slice::from_ref(&event))`.
///
/// Empty `events` is a no-op; the journal file is not created on
/// disk and `Ok(())` is returned. This lets callers compose the
/// batch unconditionally (collecting events into a `Vec` and
/// passing the slice in) without an outer `is_empty` check.
///
/// # Errors
///
/// Propagates I/O failures from the directory create / open /
/// write / fsync chain, plus JSON serialisation failures as
/// `journal-event-serialise-failed`.
pub fn append_batch(layout: Layout<'_>, events: &[Event]) -> Result<(), Error> {
    if events.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(layout.specify_dir())?;
    let path = path(layout);
    let mut payload = String::new();
    for event in events {
        let line = serde_json::to_string(event).map_err(|err| Error::Diag {
            code: "journal-event-serialise-failed",
            detail: format!("failed to serialise journal event: {err}"),
        })?;
        payload.push_str(&line);
        payload.push('\n');
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(payload.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Surface a dropped journal [`Event`] so the best-effort swallow in
/// [`super::emit_best_effort`] / [`super::emit_lint_completed`] is
/// observable and recoverable rather than silent.
///
/// Emits an operator-visible `warning:` line to stderr — matching the
/// repo's established best-effort warning idiom — and attempts to append
/// the event to the `<project_dir>/.specify/journal.dropped` sidecar (a
/// second chance at durability when the primary append failed for a
/// path-local reason). The sidecar write is itself best-effort: if it
/// too fails the stderr warning still surfaces the drop, and neither path
/// changes the calling verb's exit code or panics.
pub(super) fn record_dropped(layout: Layout<'_>, scope: &str, event: &Event, err: &Error) {
    let journal = path(layout);
    let sidecar = layout.specify_dir().join(DROPPED_FILE_NAME);
    if append_dropped(layout, event).is_ok() {
        eprintln!(
            "warning: {scope}: failed to append journal event to {} ({err}); \
             recorded the dropped event in {} for recovery",
            journal.display(),
            sidecar.display(),
        );
    } else {
        eprintln!(
            "warning: {scope}: failed to append journal event to {} ({err}); \
             the dropped event could not be written to the {} sidecar either",
            journal.display(),
            sidecar.display(),
        );
    }
}

/// Append `event` as one newline-terminated JSON line to the
/// `<project_dir>/.specify/journal.dropped` sidecar.
///
/// Mirrors [`append_batch`]'s open/append shape but is reserved for
/// events the primary journal append dropped. Returns the I/O or
/// serialisation error to the caller, which discards it ([`record_dropped`]
/// has already warned on stderr) — the helper itself never panics.
pub(super) fn append_dropped(layout: Layout<'_>, event: &Event) -> Result<(), Error> {
    let line = serde_json::to_string(event).map_err(|err| Error::Diag {
        code: "journal-event-serialise-failed",
        detail: format!("failed to serialise dropped journal event: {err}"),
    })?;
    std::fs::create_dir_all(layout.specify_dir())?;
    let sidecar = layout.specify_dir().join(DROPPED_FILE_NAME);
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&sidecar)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}
