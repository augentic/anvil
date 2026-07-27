//! Journal append plus the dropped-event sidecar recovery trail.

use std::io::Write;

use error::Error;

use super::{Event, path};
use crate::config::Layout;

/// Project-relative path of the dropped-event recovery sidecar.
pub(super) const DROPPED_FILE_NAME: &str = "journal.dropped";

/// Append a sequence of [`Event`]s to the project journal.
///
/// Events are serialized as one newline-terminated payload, appended,
/// then synced. This ordering does not make the batch transactional or
/// guarantee that the underlying writer uses one system call.
///
/// Empty batches do not create the journal file.
///
/// # Errors
///
/// Propagates I/O and JSON serialization failures.
pub fn append_batch(layout: Layout<'_>, events: &[Event]) -> Result<(), Error> {
    append_all(layout, events)
}

/// Append exactly one [`Event`], propagating failures.
///
/// # Errors
///
/// Same failure surface as [`append_batch`].
pub fn append_one(layout: Layout<'_>, event: &Event) -> Result<(), Error> {
    append_all(layout, std::slice::from_ref(event))
}

fn append_all(layout: Layout<'_>, events: &[Event]) -> Result<(), Error> {
    if events.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(layout.emery_dir())?;
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

/// Make a best-effort journal failure observable and recoverable.
///
/// The sidecar is also best-effort; stderr still reports its failure
/// without changing the calling verb's exit code.
pub(super) fn record_dropped(layout: Layout<'_>, scope: &str, event: &Event, err: &Error) {
    let journal = path(layout);
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
    let line = serde_json::to_string(event).map_err(|err| Error::Diag {
        code: "journal-event-serialise-failed",
        detail: format!("failed to serialise dropped journal event: {err}"),
    })?;
    std::fs::create_dir_all(layout.emery_dir())?;
    let sidecar = layout.emery_dir().join(DROPPED_FILE_NAME);
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&sidecar)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}
