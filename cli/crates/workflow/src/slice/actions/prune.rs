//! `archive prune` — retention GC over the slice archive.
//!
//! The archived slice folders under `.specify/archive/YYYY-MM-DD-<slice>/`
//! are a prunable convenience cache, not the system of record
//! (DECISIONS.md §"History via git plus an outcome ledger"). The durable
//! record is git history of `.specify/specs/` plus the
//! `slice.archive.created` outcome-ledger journal entries; this verb
//! reclaims disk by dropping archived folders that fall outside the
//! supplied retention bounds. Mirrors the tool-cache GC in
//! `crates/tool/src/cache/gc.rs`: a pure `scan` that computes the prune
//! set, and a `prune` that removes it.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::{Error, Result};

/// Seconds in a day, for whole-day age arithmetic.
const SECONDS_PER_DAY: i64 = 86_400;

/// Retention policy.
///
/// Both bounds are opt-in; an archived slice is retained only while it
/// satisfies **every** supplied bound, so a folder is pruned when it
/// falls outside the newest-`keep` window **or** is older than
/// `max_age_days`. With neither bound set the scan is a no-op (callers
/// reject that at the CLI boundary).
#[derive(Debug, Clone, Copy, Default)]
pub struct Retention {
    /// Keep at most this many most-recent archived slices; `None`
    /// leaves the count unbounded.
    pub keep: Option<usize>,
    /// Prune archived slices older than this many days; `None` leaves
    /// the age unbounded.
    pub max_age_days: Option<i64>,
}

/// One archived slice folder discovered under the archive directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSlice {
    /// Absolute path to the `YYYY-MM-DD-<slice>` folder.
    pub path: PathBuf,
    /// Folder basename (`YYYY-MM-DD-<slice>`).
    pub name: String,
    /// Midnight-UTC timestamp parsed from the `YYYY-MM-DD` prefix.
    pub archived_at: Timestamp,
}

/// Compute the archived slice folders that fall outside `retention`.
///
/// Entries are sorted newest-first by archive date (ties broken by name
/// descending for determinism). The returned vector preserves that
/// order and lists only the folders to prune.
///
/// # Errors
///
/// - [`Error::Filesystem`] (`op = "readdir" | "dir-entry"`) when the
///   archive directory cannot be walked.
/// - [`Error::Validation`] keyed on `archive-prune-bad-entry` when a
///   directory name does not begin with a `YYYY-MM-DD-` date prefix.
pub fn scan(
    archive_dir: &Path, retention: Retention, now: Timestamp,
) -> Result<Vec<ArchivedSlice>> {
    let mut entries = read_archive(archive_dir)?;
    // Newest first; deterministic tiebreak on name.
    entries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at).then_with(|| b.name.cmp(&a.name)));

    let mut prune = Vec::new();
    for (rank, entry) in entries.iter().enumerate() {
        let over_count = retention.keep.is_some_and(|keep| rank >= keep);
        let over_age = retention.max_age_days.is_some_and(|max| {
            let age_days = (now.as_second() - entry.archived_at.as_second()) / SECONDS_PER_DAY;
            age_days > max
        });
        if over_count || over_age {
            prune.push(entry.clone());
        }
    }
    Ok(prune)
}

/// Remove the supplied archived slice folders.
///
/// # Errors
///
/// [`Error::Filesystem`] (`op = "remove-dir-all"`) when a folder cannot
/// be removed.
pub fn prune(entries: &[ArchivedSlice]) -> Result<()> {
    for entry in entries {
        std::fs::remove_dir_all(&entry.path).map_err(|source| Error::Filesystem {
            op: "remove-dir-all",
            path: entry.path.clone(),
            source,
        })?;
    }
    Ok(())
}

fn read_archive(archive_dir: &Path) -> Result<Vec<ArchivedSlice>> {
    let mut entries = Vec::new();
    let dir = match std::fs::read_dir(archive_dir) {
        Ok(dir) => dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(source) => {
            return Err(Error::Filesystem {
                op: "readdir",
                path: archive_dir.to_path_buf(),
                source,
            });
        }
    };
    for entry in dir {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "dir-entry",
            path: archive_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let archived_at = parse_date_prefix(&name)?;
        entries.push(ArchivedSlice {
            path,
            name,
            archived_at,
        });
    }
    Ok(entries)
}

/// Parse the leading `YYYY-MM-DD` of an archive folder name into a
/// midnight-UTC timestamp.
fn parse_date_prefix(name: &str) -> Result<Timestamp> {
    let parsed = name
        .get(..10)
        .filter(|p| p.len() == 10)
        .and_then(|p| format!("{p}T00:00:00Z").parse::<Timestamp>().ok());
    parsed.ok_or_else(|| {
        Error::validation_failed(
            "archive-prune-bad-entry",
            "archive folders are named `YYYY-MM-DD-<slice>`",
            format!("archive entry `{name}` does not start with a YYYY-MM-DD date prefix"),
        )
    })
}

#[cfg(test)]
mod tests;
