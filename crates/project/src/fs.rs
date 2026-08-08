//! Narrow filesystem helpers with consistently mapped errors.
//!
//! Plain read/list sites map I/O failures onto [`Error::Filesystem`]
//! with shared `op` discriminants so call sites stay one line.

use std::fs::DirEntry;
use std::io;
use std::path::Path;

use error::{Error, Result};
use serde::Serialize;

/// Serialise `value` to a YAML document with exactly one trailing
/// newline — the shape every persisted `.yaml` artifact shares
/// (Evidence, build request/report, decision front-matter).
///
/// # Errors
///
/// [`Error::YamlSer`] when serialisation fails.
pub fn yaml<T: Serialize>(value: &T) -> Result<String> {
    let mut yaml = serde_saphyr::to_string(value)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

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

/// Move `src` to `dst`, falling back to copy-then-remove across mounts.
///
/// Uses `rename` first, then falls back on
/// [`io::ErrorKind::CrossesDevices`] (`EXDEV` / `ERROR_NOT_SAME_DEVICE`
/// — std maps the platform code) so archives on a different mount from
/// the working tree still work.
///
/// Dispatches on `src.is_dir()`: directories copy recursively, files
/// via a single `std::fs::copy`. Shared by the slice archive/discard
/// moves and the plan archive move.
///
/// # Errors
///
/// Returns `Error::Io` on rename / copy / remove failures.
pub fn move_atomic(src: &Path, dst: &Path) -> Result<()> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            if src.is_dir() {
                copy_dir_recursive(src, dst)?;
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::copy(src, dst)?;
                std::fs::remove_file(src)?;
            }
            Ok(())
        }
        Err(err) => Err(Error::Io(err)),
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_symlink() {
            let link_target = std::fs::read_link(entry.path())?;
            symlink(&link_target, &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    match std::fs::metadata(original) {
        Ok(meta) if meta.is_dir() => std::os::windows::fs::symlink_dir(original, link),
        _ => std::os::windows::fs::symlink_file(original, link),
    }
}

#[cfg(not(any(unix, windows)))]
fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "symlinks unsupported on this platform"))
}
