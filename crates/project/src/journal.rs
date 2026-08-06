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
//! A legacy `.emery/journal.jsonl` dual-write bridge keeps existing
//! single-file readers compiling until that path is retired. The
//! closed [`Event`] / [`EventKind`] taxonomy and wire DTOs live in
//! `event`; the append plus dropped-event sidecar in `append`; the
//! best-effort emit helpers in `emit`; the `emery journal show`
//! operation in [`handlers`]. Writes route through the internal
//! appenders only — CLI verbs append their own events as a side
//! effect of the operation. This root owns the read side (per-actor
//! union, forward reads, backward recent reads, and the private
//! filtered `show` projection behind `emery journal show`) and
//! re-exports the public surface so callers keep importing
//! `crate::journal::*`.
//!
//! [workflow §Observability]: ../../../../docs/standards/workflow.md#observability

mod append;
mod emit;
mod event;
pub mod handlers;

use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use error::Error;
use serde_json::Value;

pub use self::append::{append_batch, append_for, append_one};
pub use self::emit::{bracket, emit_best_effort};
pub use self::event::{AuthorityOverrideAction, Event, EventKind};
use crate::config::Layout;

/// Stable local default when `EMERY_ACTOR` is unset or empty.
pub const DEFAULT_ACTOR: &str = "local";

/// Legacy single-file journal path — dual-write bridge only.
const JOURNAL_FILE_NAME: &str = "journal.jsonl";

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

/// Absolute path to the legacy journal at
/// `<project_dir>/.emery/journal.jsonl` (dual-write bridge).
#[must_use]
pub(crate) fn path(layout: Layout<'_>) -> PathBuf {
    layout.emery_dir().join(JOURNAL_FILE_NAME)
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

/// Read every parseable [`Event`] from the legacy journal at
/// `<project_dir>/.emery/journal.jsonl`, in append (file) order.
///
/// Bridge reader for `journal show` / identity projections until they
/// retarget to [`read_union`]. A missing journal yields an empty
/// vector. Blank and unparseable lines are skipped.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub(crate) fn read(layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    read_file(&path(layout))
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

/// Byte window the backward tail reader pulls per `read`/`seek`. One
/// `O_APPEND` journal line stays well under this, so the common case of a
/// few recent matches resolves in a single window.
const TAIL_CHUNK: usize = 8192;

/// Read the most recent journal [`Event`]s that `select` maps to a value,
/// returning at most `limit` of them in append (file) order.
///
/// Tails the legacy journal backward (via the private `for_each_line_rev`)
/// and stops as soon as `limit` matches are collected. Bridge reader —
/// union-aware recent reads land when `journal show` retargets.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub(crate) fn read_recent<T>(
    layout: Layout<'_>, limit: usize, mut select: impl FnMut(Event) -> Option<T>,
) -> Result<Vec<T>, Error> {
    let mut newest_first: Vec<T> = Vec::new();
    if limit == 0 {
        return Ok(newest_first);
    }
    for_each_line_rev(&path(layout), TAIL_CHUNK, |line| {
        if line.trim().is_empty() {
            return true;
        }
        if let Ok(event) = serde_json::from_str::<Event>(line)
            && let Some(item) = select(event)
        {
            newest_first.push(item);
            if newest_first.len() >= limit {
                return false;
            }
        }
        true
    })
    .map_err(Error::Io)?;
    newest_first.reverse();
    Ok(newest_first)
}

/// Visit journal [`Event`]s newest-first until `visit` breaks or the
/// head of the file is reached.
///
/// Bridge reader over the legacy single-file journal.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub(crate) fn scan_recent(
    layout: Layout<'_>, mut visit: impl FnMut(Event) -> std::ops::ControlFlow<()>,
) -> Result<(), Error> {
    for_each_line_rev(&path(layout), TAIL_CHUNK, |line| {
        if line.trim().is_empty() {
            return true;
        }
        serde_json::from_str::<Event>(line).map_or(true, |event| visit(event).is_continue())
    })
    .map_err(Error::Io)
}

/// Read events for `emery journal show`, in append (file) order.
///
/// `filter` keeps events whose dotted-kebab wire id starts with the
/// given prefix (e.g. `slice.build` or `plan.entry.advanced`); `limit`
/// keeps only the most recent N matches, tailing via [`read_recent`]
/// so the bytes touched stay bounded by the limit rather than total
/// history. Reader leniency matches [`read`]: blank and unparseable
/// lines are skipped and a missing journal yields an empty vector.
/// Private: the only consumer is the [`handlers::Show`] handler.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
fn show(
    layout: Layout<'_>, filter: Option<&str>, limit: Option<usize>,
) -> Result<Vec<Event>, Error> {
    let keep = |event: &Event| filter.is_none_or(|prefix| wire_id(&event.kind).starts_with(prefix));
    match limit {
        Some(limit) => read_recent(layout, limit, |event| keep(&event).then_some(event)),
        None => Ok(read(layout)?.into_iter().filter(keep).collect()),
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

/// Visit the complete lines of the file at `path` newest-first, invoking
/// `visit` for each; `visit` returns `false` to stop early (the unread
/// head of the file is then never read).
///
/// The file is read backward in `chunk`-byte windows, so only the tail the
/// consumer scans is touched. Line boundaries follow [`str::lines`]: a
/// single trailing newline is a terminator (no empty final line) while
/// interior blank lines are preserved. Splitting happens on `b'\n'`
/// boundaries — multi-byte UTF-8 sequences spanning a chunk edge are
/// reassembled before decoding, and every emitted line spans from just
/// after a newline (or file start) to just before the next newline (or
/// end), which are always character boundaries in a valid UTF-8 journal.
///
/// A missing file yields no visits (`Ok(())`), mirroring [`read`].
fn for_each_line_rev(
    path: &Path, chunk: usize, mut visit: impl FnMut(&str) -> bool,
) -> std::io::Result<()> {
    debug_assert!(chunk > 0, "tail chunk size must be non-zero");
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    let mut pos = file.seek(SeekFrom::End(0))?;
    if pos == 0 {
        return Ok(());
    }
    let chunk = u64::try_from(chunk).unwrap_or(u64::MAX);
    // `carry` holds the leading partial segment of the window read so far
    // (the bytes before its first newline); its true start lies in an
    // as-yet-unread earlier chunk, so it is only decoded once `pos` hits 0.
    let mut carry: Vec<u8> = Vec::new();
    let mut first = true;
    while pos > 0 {
        let take = pos.min(chunk);
        pos -= take;
        file.seek(SeekFrom::Start(pos))?;
        let mut window = vec![0_u8; usize::try_from(take).unwrap_or(usize::MAX)];
        file.read_exact(&mut window)?;
        window.extend_from_slice(&carry);
        if first {
            first = false;
            // Drop a single trailing newline so a terminator does not yield
            // an empty final line (str::lines parity).
            if window.last() == Some(&b'\n') {
                window.pop();
            }
        }
        // Emit every line after the first newline (newest-first); retain
        // the pre-first-newline head as the next `carry`.
        while let Some(idx) = window.iter().rposition(|&byte| byte == b'\n') {
            let keep_going = visit(String::from_utf8_lossy(&window[idx + 1..]).as_ref());
            window.truncate(idx);
            if !keep_going {
                return Ok(());
            }
        }
        carry = window;
    }
    // `pos == 0`: the remaining bytes are the file's first line.
    visit(String::from_utf8_lossy(&carry).as_ref());
    Ok(())
}
