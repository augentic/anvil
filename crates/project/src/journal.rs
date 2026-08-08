//! Workflow journal events: append-only newline-delimited JSON at
//! `.emery/events/<writer>.jsonl`. Each writer appends only its own
//! file; readers union all files by `(timestamp, writer, sequence)`.

mod append;
pub mod claim;
mod emit;
mod event;
pub mod handlers;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use error::Error;
use serde_json::Value;

pub use self::append::{append_batch, append_for, append_one};
pub use self::emit::{bracket, emit_best_effort};
pub use self::event::{
    AuthorityOverrideAction, ClosedPlanCoverage, Event, EventKind, FactEpochRef, IdentityMap,
    LeafSpecCoverage, UnknownWaiver,
};
use crate::config::Layout;

/// Stable local default when `EMERY_WRITER` is unset or empty.
pub const DEFAULT_WRITER: &str = "local";

/// Resolve the calling writer id at the journal-append boundary.
///
/// A non-empty `EMERY_WRITER` wins; otherwise [`DEFAULT_WRITER`]. The
/// wasm32 guest has no process environment for this variable and
/// always uses the default — pass an explicit writer to [`append_for`]
/// when a guest-side identity is required.
#[must_use]
pub fn writer_id() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(value) = std::env::var("EMERY_WRITER") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    DEFAULT_WRITER.to_string()
}

/// Absolute path to one writer's log at
/// `<project_dir>/.emery/events/<writer>.jsonl`.
#[must_use]
pub(crate) fn writer_log_path(layout: Layout<'_>, writer: &str) -> PathBuf {
    layout.writer_events_path(writer)
}

/// Refuse writer ids that cannot be a single path segment under
/// `.emery/events/`.
pub(crate) fn validate_writer(writer: &str) -> Result<(), Error> {
    if writer.is_empty()
        || writer == "."
        || writer == ".."
        || writer.contains('/')
        || writer.contains('\\')
        || writer.contains('\0')
    {
        return Err(Error::Diag {
            code: "journal-writer-invalid",
            detail: format!(
                "writer id {writer:?} must be a non-empty single path segment \
                 (no `/`, `\\`, or NUL)"
            ),
        });
    }
    Ok(())
}

/// Read every parseable [`Event`] from every per-writer log under
/// `.emery/events/`, ordered by `(timestamp, writer, sequence)`.
///
/// A missing events directory yields an empty vector. Blank lines and
/// lines that fail to parse as an [`Event`] are skipped rather than
/// failing the whole read, so a log written by a newer binary still
/// yields the events this binary understands.
///
/// # Errors
///
/// Propagates I/O failures other than a missing events directory.
pub fn read_union(layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    let dir = layout.events_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::Io(err)),
    };
    let mut events = Vec::new();
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        events.extend(read_file(&path)?);
    }
    events.sort_by(|left, right| {
        (left.timestamp, left.writer.as_str(), left.sequence).cmp(&(
            right.timestamp,
            right.writer.as_str(),
            right.sequence,
        ))
    });
    Ok(events)
}

fn read_file(path: &Path) -> Result<Vec<Event>, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::Io(err)),
    };
    Ok(contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Event>(line).ok())
        .collect())
}

/// Read the most recent journal [`Event`]s that `select` maps to a value,
/// returning at most `limit` of them in union order.
///
/// Loads the per-writer union ([`read_union`]) and keeps the newest
/// matching events. Cost tracks total event count; the union is the
/// authority, so a single-file reverse-tail is not a substitute.
///
/// # Errors
///
/// Propagates I/O failures other than a missing events directory.
pub(crate) fn read_recent<T>(
    layout: Layout<'_>, limit: usize, mut select: impl FnMut(Event) -> Option<T>,
) -> Result<Vec<T>, Error> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut newest_first: Vec<T> = Vec::new();
    for event in read_union(layout)?.into_iter().rev() {
        if let Some(item) = select(event) {
            newest_first.push(item);
            if newest_first.len() >= limit {
                break;
            }
        }
    }
    newest_first.reverse();
    Ok(newest_first)
}

/// Read events for `emery journal show`, in union order
/// (`timestamp`, `writer`, `sequence`).
///
/// `filter` keeps events whose dotted-kebab wire id starts with the
/// given prefix (e.g. `slice.build` or `plan.entry.advanced`); `limit`
/// keeps only the most recent N matches via [`read_recent`]. Reader
/// leniency matches [`read_union`]: blank and unparseable lines are
/// skipped and a missing events directory yields an empty vector.
/// Private: the only consumer is the [`handlers::Show`] handler.
///
/// # Errors
///
/// Propagates I/O failures other than a missing events directory.
fn show(
    layout: Layout<'_>, filter: Option<&str>, limit: Option<usize>,
) -> Result<Vec<Event>, Error> {
    let keep = |event: &Event| filter.is_none_or(|prefix| wire_id(&event.kind).starts_with(prefix));
    match limit {
        Some(limit) => read_recent(layout, limit, |event| keep(&event).then_some(event)),
        None => Ok(read_union(layout)?.into_iter().filter(keep).collect()),
    }
}

/// Dotted-kebab wire id of `kind`, read back from its serde tag so the
/// adjacently-tagged wire shape stays the single source of truth (no
/// hand-maintained per-variant match to drift). [`EventKind`] always
/// serialises, so the fallback empty string (which matches no filter
/// prefix) is unreachable in practice.
fn wire_id(kind: &EventKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.get("event").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default()
}
