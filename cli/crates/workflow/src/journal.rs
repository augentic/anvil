//! Workflow journal events.
//!
//! Append-only newline-delimited JSON at `.specify/journal.jsonl`,
//! shared by every plan-, slice-, propose-, extract-, and synthesis-
//! related signal listed in [workflow §Observability]. One line per
//! [`Event`]; readers tail the file and skip blank lines.
//!
//! The closed [`Event`] / [`EventKind`] taxonomy and wire DTOs live in
//! `event`; the append plus dropped-event sidecar in `append`; the
//! best-effort emit helpers in `emit`. This root owns the read side
//! (forward [`read`], backward [`read_recent`], and the filtered
//! [`show`] projection behind `specify journal show`) and re-exports
//! the public surface so callers keep importing `crate::journal::*`.
//!
//! [workflow §Observability]: ../../../../docs/standards/workflow.md#observability

mod append;
mod emit;
mod event;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_shapes;

use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;
use specify_error::Error;

pub use self::append::append_batch;
pub use self::emit::{emit_best_effort, emit_lint_completed};
pub use self::event::{
    Actor, AuthorityOverrideAction, Event, EventKind, LintCompletedPayload, LintCounts, LintScope,
    WIRE_EVENT_IDS,
};
use crate::config::Layout;

/// Project-relative path the journal lives at.
const JOURNAL_FILE_NAME: &str = "journal.jsonl";

/// Absolute path to the journal at `<project_dir>/.specify/journal.jsonl`.
#[must_use]
pub fn path(layout: Layout<'_>) -> PathBuf {
    layout.specify_dir().join(JOURNAL_FILE_NAME)
}

/// Read every parseable [`Event`] from the journal at
/// `<project_dir>/.specify/journal.jsonl`, in append (file) order.
///
/// A missing journal yields an empty vector. Blank lines are skipped.
/// Lines that fail to parse as an [`Event`] are skipped rather than
/// failing the whole read, so a journal written by a newer binary
/// (carrying event kinds this binary does not know) still yields the
/// events it does understand — the read stays forward-compatible and,
/// for a given file, deterministic.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub fn read(layout: Layout<'_>) -> Result<Vec<Event>, Error> {
    let path = path(layout);
    let contents = match std::fs::read_to_string(&path) {
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
/// Tails the journal backward (via the private `for_each_line_rev`) and stops as
/// soon as `limit` matches are collected, so the bytes touched are bounded
/// by how far back the `limit`-th match sits — not by total history. This
/// keeps the projection cost flat as the journal grows.
///
/// Blank lines are skipped and lines that fail to parse as an [`Event`]
/// are skipped rather than failing the read — identical leniency to
/// [`read`], so a journal written by a newer binary still yields the
/// matches this binary understands. A missing journal yields an empty
/// vector. This is the read side the identity projection
/// (`recent[]`) and [`show`] consume.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub fn read_recent<T>(
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

/// Read events for `specify journal show`, in append (file) order.
///
/// `filter` keeps events whose dotted-kebab wire id starts with the
/// given prefix (e.g. `slice.build` or `plan.entry.advanced`); `limit`
/// keeps only the most recent N matches, tailing via [`read_recent`]
/// so the bytes touched stay bounded by the limit rather than total
/// history. Reader leniency matches [`read`]: blank and unparseable
/// lines are skipped and a missing journal yields an empty vector.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file.
pub fn show(
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

/// Parses a fixed RFC3339 timestamp for test fixtures.
#[cfg(test)]
pub(crate) fn test_timestamp(raw: &str) -> jiff::Timestamp {
    raw.parse().expect("valid rfc3339 timestamp in test fixture")
}
