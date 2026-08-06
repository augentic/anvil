//! Workflow journal events.
//!
//! Append-only newline-delimited JSON at `.emery/events/<actor>.jsonl`
//! (RFC-86 D3). Each actor appends only their own file; readers union
//! every actor file ordered by `(timestamp, actor, sequence)`.
//!
//! **Actor id.** The calling actor is `EMERY_ACTOR` when that
//! environment variable is set to a non-empty value; otherwise the
//! stable local default [`DEFAULT_ACTOR`] (`"local"`). Multi-actor
//! fixtures pass an explicit id to [`append_for`] instead of reading
//! the environment. Until the RFC-88 two-root cut, these logs live
//! under the flat `.emery/events/` stand-in (not `.emery/change/events/`).
//!
//! The closed [`Event`] / [`EventKind`] taxonomy and wire DTOs live in
//! `event`; the append plus dropped-event sidecar in `append`; the
//! exclusive per-slice claim projection in [`claim`] (RFC-86 D7 /
//! D23); the best-effort emit helpers in `emit`; the `emery journal
//! show` operation in [`handlers`]. Writes route through the internal
//! appenders only — CLI verbs append their own events as a side
//! effect of the operation. This root owns the read side (per-actor
//! union, recent-tail projection, and the private filtered `show`
//! projection behind `emery journal show`) and re-exports the public
//! surface so callers keep importing `crate::journal::*`.
//!
//! [workflow §Observability]: ../../../../docs/standards/workflow.md#observability

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
pub use self::event::{AuthorityOverrideAction, Event, EventKind};
use crate::config::Layout;

/// Stable local default when `EMERY_ACTOR` is unset or empty.
pub const DEFAULT_ACTOR: &str = "local";

/// Resolve the calling actor id at the journal-append boundary.
///
/// A non-empty `EMERY_ACTOR` wins; otherwise [`DEFAULT_ACTOR`]. The
/// wasm32 guest has no process environment for this variable and
/// always uses the default — pass an explicit actor to [`append_for`]
/// when a guest-side identity is required.
#[must_use]
pub fn actor_id() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(value) = std::env::var("EMERY_ACTOR") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    DEFAULT_ACTOR.to_string()
}

/// Absolute path to one actor's log at
/// `<project_dir>/.emery/events/<actor>.jsonl`.
#[must_use]
pub(crate) fn actor_log_path(layout: Layout<'_>, actor: &str) -> PathBuf {
    layout.actor_events_path(actor)
}

/// Refuse actor ids that cannot be a single path segment under
/// `.emery/events/`.
pub(crate) fn validate_actor(actor: &str) -> Result<(), Error> {
    if actor.is_empty()
        || actor == "."
        || actor == ".."
        || actor.contains('/')
        || actor.contains('\\')
        || actor.contains('\0')
    {
        return Err(Error::Diag {
            code: "journal-actor-invalid",
            detail: format!(
                "actor id {actor:?} must be a non-empty single path segment \
                 (no `/`, `\\`, or NUL)"
            ),
        });
    }
    Ok(())
}

/// Read every parseable [`Event`] from every per-actor log under
/// `.emery/events/`, ordered by `(timestamp, actor, sequence)`.
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
        (left.timestamp, left.actor.as_str(), left.sequence).cmp(&(
            right.timestamp,
            right.actor.as_str(),
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
/// Loads the per-actor union ([`read_union`]) and keeps the newest
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
/// (`timestamp`, `actor`, `sequence`).
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
