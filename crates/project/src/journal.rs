//! Workflow journal events.
//!
//! Append-only NDJSON per writer under a [`JournalRoot`]; readers
//! union by `(timestamp, writer, sequence)`.

mod append;
pub mod claim;
mod emit;
mod event;
pub mod handlers;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use error::Error;
use serde_json::Value;

pub use self::append::{append_batch, append_for, append_for_at, append_one};
pub use self::emit::{bracket, emit_best_effort};
pub use self::event::{
    AuthorityOverrideAction, ClosedPlanCoverage, ClosedReason, DeferredMember, Event, EventKind,
    FactEpochRef, IdentityMap, ParkReason,
};
use crate::config::Layout;

/// Stable local default when `EMERY_WRITER` is unset or empty.
pub const DEFAULT_WRITER: &str = "local";

/// One journal's events directory.
///
/// The product change home journals at `.emery/change/events/`; an
/// RFC-104 definition home journals at `<system>/events/`. Carrying
/// the events directory instead of a product [`Layout`] keeps the two
/// roots separate while sharing the append/read kernels and the
/// RFC-86 writer/sequence union semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRoot {
    events_dir: PathBuf,
}

impl JournalRoot {
    /// Anchor a journal at an explicit events directory.
    #[must_use]
    pub const fn new(events_dir: PathBuf) -> Self {
        Self { events_dir }
    }

    /// The directory holding the per-writer `.jsonl` logs.
    #[must_use]
    pub fn events_dir(&self) -> &Path {
        &self.events_dir
    }

    /// `<events>/<writer>.jsonl` — one writer's log.
    #[must_use]
    pub fn writer_path(&self, writer: &str) -> PathBuf {
        self.events_dir.join(format!("{writer}.jsonl"))
    }
}

impl From<Layout<'_>> for JournalRoot {
    fn from(layout: Layout<'_>) -> Self {
        Self::new(layout.events_dir())
    }
}

/// Resolve the calling writer id at the journal-append boundary.
///
/// A non-empty `EMERY_WRITER` wins; otherwise [`DEFAULT_WRITER`]. The
/// launcher exposes the host environment to the wasm guest through the
/// same path as `HTTP_ADDR` (RFC-96 D4), so guest and native reads
/// agree. Writer identity is deployment configuration, never a
/// capability.
#[must_use]
pub fn writer_id() -> String {
    if let Ok(value) = std::env::var("EMERY_WRITER") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    DEFAULT_WRITER.to_string()
}

/// Absolute path to one writer's log at
/// `<project_dir>/.emery/change/events/<writer>.jsonl`.
#[must_use]
pub(crate) fn writer_log_path(layout: Layout<'_>, writer: &str) -> PathBuf {
    layout.writer_events_path(writer)
}

/// Export the JournalRoot-based union read beside the Layout one.
///
/// # Errors
///
/// Same failure surface as [`read_union`].
pub fn read_union_at(root: &JournalRoot) -> Result<Vec<Event>, Error> {
    read_union_dir(root.events_dir(), Parse::Strict)
}

/// Refuse writer ids that cannot be a single path segment under
/// `.emery/change/events/`.
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

/// Read every [`Event`] from every per-writer log under
/// `.emery/change/events/`, ordered by `(timestamp, writer, sequence)`.
///
/// A missing events directory yields an empty vector. The read is
/// **strict** (S13 / CC-11): a non-empty line that fails to parse is a
/// typed `journal-line-malformed` error, never silently skipped — an
/// authority-forming read must not project partial state from a
/// corrupt log. The read-only `emery journal show` projection uses the
/// lenient variant instead.
///
/// # Errors
///
/// `journal-line-malformed` on a corrupt line; I/O failures other than
/// a missing events directory.
pub fn read_union(layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    read_union_dir(&layout.events_dir(), Parse::Strict)
}

/// Line handling for the union read: authority reads are strict,
/// observability projections are lenient.
#[derive(Clone, Copy)]
enum Parse {
    Strict,
    Lenient,
}

/// The union read over one explicit events directory.
fn read_union_dir(dir: &Path, parse: Parse) -> Result<Vec<Event>, Error> {
    let entries = match std::fs::read_dir(dir) {
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
        events.extend(read_file(&path, parse)?);
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

fn read_file(path: &Path, parse: Parse) -> Result<Vec<Event>, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::Io(err)),
    };
    let mut events = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(line) {
            Ok(event) => events.push(event),
            Err(err) => match parse {
                Parse::Strict => {
                    return Err(Error::Diag {
                        code: "journal-line-malformed",
                        detail: format!(
                            "{}:{}: {err} — repair or remove the corrupt journal line",
                            path.display(),
                            index + 1
                        ),
                    });
                }
                Parse::Lenient => {}
            },
        }
    }
    Ok(events)
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
/// given prefix; `limit` keeps only the most recent N matches. The one
/// **lenient** reader: blank and unparseable lines are skipped so the
/// observability projection still shows what this binary understands —
/// authority reads use the strict [`read_union`] instead.
///
/// # Errors
///
/// Propagates I/O failures other than a missing events directory.
fn show(
    layout: Layout<'_>, filter: Option<&str>, limit: Option<usize>,
) -> Result<Vec<Event>, Error> {
    let keep = |event: &Event| filter.is_none_or(|prefix| wire_id(&event.kind).starts_with(prefix));
    let mut events: Vec<Event> = read_union_dir(&layout.events_dir(), Parse::Lenient)?
        .into_iter()
        .filter(|event| keep(event))
        .collect();
    if let Some(limit) = limit {
        let start = events.len().saturating_sub(limit);
        events.drain(..start);
    }
    Ok(events)
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
