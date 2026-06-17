//! Crash-safe writers shared by every `.specify/*.yaml` writer: write
//! to a temp file in the same parent, `sync_all`, then `persist`
//! (atomic rename) so readers never observe a partial write.

use std::path::Path;

use serde::Serialize;
use specify_error::Error;

/// Serialise `value` as YAML (with a guaranteed trailing newline) and
/// atomically persist it at `path`. See module-level docs for the
/// atomicity envelope.
///
/// # Errors
///
/// Returns `Error::YamlSer` if serialisation fails, or `Error::Io` if
/// the temp-file write or rename fails.
pub fn yaml_write<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    bytes_write(path, serialise_yaml(value)?.as_bytes())
}

/// Serialise `value` as a YAML document with a guaranteed single
/// trailing newline, returning the string rather than writing it.
///
/// # Errors
///
/// Returns `Error::YamlSer` if serialisation fails.
pub fn serialise_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut content = serde_saphyr::to_string(value)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}

/// Atomically write `bytes` to `path`. Used for non-YAML writers (e.g.
/// the PID stamp in `.specify/plan.lock`) where the caller has already
/// produced the exact on-disk bytes.
///
/// # Errors
///
/// Returns `Error::Io` if the temp-file create / write / rename fails.
pub fn bytes_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(tmp.as_file_mut(), bytes)?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde::{Deserialize, Serialize};

    use super::{bytes_write, yaml_write};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Doc {
        name: String,
        count: u32,
    }

    /// A write into a not-yet-existing nested directory creates the
    /// parent chain and lands the exact bytes.
    #[test]
    fn creates_missing_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("nested").join("deeper").join("out.bin");
        bytes_write(&target, b"payload").expect("write");
        assert_eq!(fs::read(&target).expect("read"), b"payload");
    }

    /// A second write fully replaces the file — a shorter payload must
    /// not leave a tail of the longer original behind (atomic rename,
    /// not in-place truncate-then-write).
    #[test]
    fn overwrite_replaces_whole_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.bin");
        bytes_write(&target, b"AAAAAAAAAAAA").expect("first write");
        bytes_write(&target, b"BB").expect("second write");
        assert_eq!(fs::read(&target).expect("read"), b"BB");
    }

    /// A successful write leaves only the target in the parent — the
    /// `NamedTempFile` is consumed by `persist`, never orphaned.
    #[test]
    fn leaves_no_temp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.bin");
        bytes_write(&target, b"x").expect("write");
        let entries: Vec<_> = fs::read_dir(dir.path())
            .expect("read_dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("out.bin")]);
    }

    /// YAML serialisation round-trips and the file ends with exactly one
    /// newline regardless of what `serde_saphyr` emits.
    #[test]
    fn yaml_round_trips_trailing_newline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("doc.yaml");
        let doc = Doc {
            name: "alpha".into(),
            count: 3,
        };
        yaml_write(&target, &doc).expect("write");

        let raw = fs::read_to_string(&target).expect("read");
        assert!(raw.ends_with('\n'), "trailing newline present, got {raw:?}");
        assert!(!raw.ends_with("\n\n"), "newline not doubled, got {raw:?}");
        let parsed: Doc = serde_saphyr::from_str(&raw).expect("parse");
        assert_eq!(parsed, doc);
    }
}
