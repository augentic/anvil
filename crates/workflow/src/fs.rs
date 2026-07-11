//! Narrow filesystem helpers with consistently mapped errors.
//!
//! Every plain "read this file" / "list this directory" site in the
//! workflow crate maps I/O failures onto [`Error::Filesystem`] with the
//! same `op` discriminants (`read` / `readdir`); these helpers own that
//! mapping so call sites stay one line. Sites with richer semantics
//! (missing-file fallbacks, symlink policy, drift detection) keep their
//! own handling.

use std::fs::DirEntry;
use std::path::Path;

use error::{Error, Result};

/// Read `path` to a string, mapping the failure onto
/// [`Error::Filesystem`] with `op: "read"`.
///
/// # Errors
///
/// [`Error::Filesystem`] when the file cannot be read.
pub fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })
}

/// Collect `dir`'s entries, mapping both the open and the per-entry
/// failures onto [`Error::Filesystem`] with `op: "readdir"`.
///
/// # Errors
///
/// [`Error::Filesystem`] when the directory cannot be opened or an
/// entry cannot be read.
pub fn dir_entries(dir: &Path) -> Result<Vec<DirEntry>> {
    let read_dir = std::fs::read_dir(dir).map_err(|source| Error::Filesystem {
        op: "readdir",
        path: dir.to_path_buf(),
        source,
    })?;
    read_dir
        .map(|entry| {
            entry.map_err(|source| Error::Filesystem {
                op: "readdir",
                path: dir.to_path_buf(),
                source,
            })
        })
        .collect()
}
