//! Crash-safe same-directory temp writes: write, sync, then rename.

use std::path::Path;

use emery_error::Error;
use serde::Serialize;

/// Atomically write `value` as YAML with a trailing newline.
///
/// # Errors
///
/// Propagates serialization and filesystem failures.
pub fn yaml_write<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    bytes_write(path, serialise_yaml(value)?.as_bytes())
}

/// Serialise `value` as YAML with a trailing newline.
///
/// # Errors
///
/// Returns an error when YAML serialization fails.
pub fn serialise_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut content = serde_saphyr::to_string(value)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}

/// Atomically write exact `bytes` to `path`.
///
/// # Errors
///
/// Propagates directory, temporary-file, write, sync, and persist failures.
pub fn bytes_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(tmp.as_file_mut(), bytes)?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}

/// Atomically stream-copy `src` to `path`.
///
/// # Errors
///
/// Propagates directory, temporary-file, read, write, sync, and persist failures.
pub fn copy_write(path: &Path, src: &Path) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    let mut from = std::fs::File::open(src)?;
    std::io::copy(&mut from, tmp.as_file_mut())?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
