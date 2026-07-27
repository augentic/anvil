//! Crash-safe writers shared by every `.emery/*.yaml` writer: write
//! to a temp file in the same parent, `sync_all`, then `persist`
//! (atomic rename) so readers never observe a partial write.

use std::path::Path;

use error::Error;
use serde::Serialize;

/// Serialise `value` as YAML (with a guaranteed trailing newline) and
/// atomically persist it at `path`. See module-level docs for the
/// atomicity envelope.
///
/// # Errors
///
/// Propagates serialization and filesystem failures.
pub fn yaml_write<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    bytes_write(path, serialise_yaml(value)?.as_bytes())
}

/// Serialise `value` as a YAML document with a guaranteed single
/// trailing newline, returning the string rather than writing it.
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

/// Atomically write `bytes` to `path`. Used for non-YAML writers (e.g.
/// the PID stamp in `.emery/plan.lock`) where the caller has already
/// produced the exact on-disk bytes.
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
