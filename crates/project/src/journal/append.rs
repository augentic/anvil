//! Per-writer journal append plus the dropped-event sidecar recovery trail.

use std::io::{ErrorKind, Write};
use std::path::Path;

use error::Error;

use super::{Event, validate_writer, writer_log_path};
use crate::config::Layout;

/// Project-relative path of the dropped-event recovery sidecar.
pub(super) const DROPPED_FILE_NAME: &str = "journal.dropped";

/// Append a sequence of [`Event`]s to the calling writer's log.
///
/// Resolves the writer via [`super::writer_id`], stamps monotonic
/// `writer` / `sequence` on each line, and writes only that writer's
/// `.emery/change/events/<writer>.jsonl` file.
///
/// Empty batches do not create the file.
///
/// # Errors
///
/// Propagates I/O and JSON serialization failures, and
/// `journal-writer-invalid` when the resolved writer id is not a safe
/// single path segment.
pub fn append_batch(layout: Layout<'_>, events: &[Event]) -> Result<(), Error> {
    append_for(layout, &super::writer_id(), events)
}

/// Append exactly one [`Event`], propagating failures.
///
/// # Errors
///
/// Same failure surface as [`append_batch`].
pub fn append_one(layout: Layout<'_>, event: &Event) -> Result<(), Error> {
    append_for(layout, &super::writer_id(), std::slice::from_ref(event))
}

/// Append `events` to one explicit writer's log.
///
/// Primary write surface for multi-writer fixtures and any caller that
/// already knows the writer id. Stamps monotonic `sequence` values
/// (1-based) continuing from the last line in that writer's file.
///
/// # Errors
///
/// Propagates I/O and JSON serialization failures, and
/// `journal-writer-invalid` when `writer` is empty or contains a path
/// separator.
pub fn append_for(layout: Layout<'_>, writer: &str, events: &[Event]) -> Result<(), Error> {
    validate_writer(writer)?;
    if events.is_empty() {
        return Ok(());
    }
    let writer_path = writer_log_path(layout, writer);
    if let Some(parent) = writer_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut next = next_sequence(&writer_path)?;
    let mut payload = String::new();
    for event in events {
        let stamped = Event {
            timestamp: event.timestamp,
            writer: writer.to_string(),
            sequence: next,
            kind: event.kind.clone(),
        };
        next = next.saturating_add(1);
        let line = serialize_event(&stamped)?;
        payload.push_str(&line);
        payload.push('\n');
    }
    append_bytes(&writer_path, payload.as_bytes())?;
    Ok(())
}

fn next_sequence(path: &Path) -> Result<u64, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(1),
        Err(err) => return Err(Error::Io(err)),
    };
    let last = contents
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str::<Event>(line).ok());
    Ok(last.map_or(1, |event| event.sequence.saturating_add(1)))
}

fn serialize_event(event: &Event) -> Result<String, Error> {
    serde_json::to_string(event).map_err(|err| Error::Diag {
        code: "journal-event-serialise-failed",
        detail: format!("failed to serialise journal event: {err}"),
    })
}

fn append_bytes(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Make a best-effort journal failure observable and recoverable.
///
/// The sidecar is also best-effort; stderr still reports its failure
/// without changing the calling verb's exit code.
pub(super) fn record_dropped(layout: Layout<'_>, scope: &str, event: &Event, err: &Error) {
    let writer = if event.writer.is_empty() { super::writer_id() } else { event.writer.clone() };
    let journal = writer_log_path(layout, &writer);
    let sidecar = layout.emery_dir().join(DROPPED_FILE_NAME);
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

/// Append `event` to the dropped-event recovery sidecar.
pub(super) fn append_dropped(layout: Layout<'_>, event: &Event) -> Result<(), Error> {
    let line = serialize_event(event)?;
    std::fs::create_dir_all(layout.emery_dir())?;
    let sidecar = layout.emery_dir().join(DROPPED_FILE_NAME);
    append_bytes(&sidecar, format!("{line}\n").as_bytes())
}
