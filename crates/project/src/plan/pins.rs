//! Source `cid` pins recorded during `plan author`.
//!
//! Each [`super::SourceBinding`] records its input tree's identity
//! (tree-manifest digest encoding); the store is never populated here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::Error;

use super::model::{Plan, SourceBinding};
use crate::snapshot::{self, SnapshotId};

/// Entry path used for the one-file tree of a value binding.
const VALUE_ENTRY: &str = "content";

/// Close per-source `cid` pins on `plan.sources` against `project_root`.
///
/// Called once the authoring source set is known (after scaffold +
/// survey). Overwrites any prior `cid` so `--force` re-authoring
/// re-pins.
///
/// # Errors
///
/// `source-pin-unbound` when a binding carries neither `locator` nor
/// `value`; `source-pin-missing` when a locator binding's target is
/// absent; filesystem / path-shape failures from the tree walk.
pub fn close(plan: &mut Plan, project_root: &Path) -> Result<(), Error> {
    for (key, binding) in &mut plan.sources {
        if binding.value.is_some() {
            binding.cid = None;
            continue;
        }
        binding.cid = Some(cid_for(key, binding, project_root)?);
    }
    Ok(())
}

/// Content-addressed identity of an inline value binding.
#[must_use]
pub fn value_cid(value: &str) -> SnapshotId {
    file_cid(VALUE_ENTRY, value.as_bytes())
}

/// Digest of a one-file tree whose sole entry is `path` → `bytes`.
#[must_use]
pub fn file_cid(path: &str, bytes: &[u8]) -> SnapshotId {
    let blob = sha256_hex(bytes);
    let mut entries = BTreeMap::new();
    entries.insert(path.to_string(), Entry::File { exec: false, blob });
    SnapshotId::from_digest(&sha256_hex(encode(&entries).as_bytes()))
}

/// Content-addressed identity of a directory tree (digest-only).
///
/// Absent or non-directory paths share the empty-tree identity so a
/// greenfield baseline `specs/` (not yet created) pins stably. Digests
/// match [`crate::workspace::Store::snapshot`] for ordinary directory
/// trees — the refinement manifest's `baseline-specs` pin uses this.
///
/// # Errors
///
/// Filesystem / path-shape failures from the tree walk.
pub fn dir_cid(dir: &Path) -> Result<SnapshotId, Error> {
    if !dir.is_dir() {
        return Ok(empty_cid());
    }
    tree_cid(dir)
}

/// Identity of the empty tree manifest.
#[must_use]
pub fn empty_cid() -> SnapshotId {
    SnapshotId::from_digest(&sha256_hex(encode(&BTreeMap::new()).as_bytes()))
}

/// Live content-addressed identity of one source binding.
///
/// Same digest as [`close`] would stamp onto `binding.cid`. Used by
/// refinement freshness to compare recorded source pins against the
/// current locator/value tree without rewriting the plan.
///
/// # Errors
///
/// Same taxonomy as [`close`] for a single binding.
pub fn source_cid(
    key: &str, binding: &SourceBinding, project_root: &Path,
) -> Result<SnapshotId, Error> {
    cid_for(key, binding, project_root)
}

fn cid_for(key: &str, binding: &SourceBinding, project_root: &Path) -> Result<SnapshotId, Error> {
    if binding.value.is_some() {
        return Err(Error::Diag {
            code: "source-pin-unbound",
            detail: format!("source `{key}` is an inline value; it does not close a tree cid pin"),
        });
    }
    if let Some(locator) = binding.locator.as_deref() {
        return path_cid(key, locator, project_root);
    }
    Err(Error::Diag {
        code: "source-pin-unbound",
        detail: format!(
            "source `{key}` has neither `locator` nor `value`; cannot close a tree cid pin"
        ),
    })
}

fn path_cid(key: &str, raw: &str, project_root: &Path) -> Result<SnapshotId, Error> {
    let path = resolve(project_root, raw);
    let meta = std::fs::symlink_metadata(&path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            Error::Diag {
                code: "source-pin-missing",
                detail: format!(
                    "source `{key}` path `{raw}` does not exist; cannot close a tree cid pin"
                ),
            }
        } else {
            Error::Filesystem {
                op: "stat",
                path: path.clone(),
                source,
            }
        }
    })?;
    if meta.file_type().is_symlink() || meta.is_file() {
        let bytes = std::fs::read(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| unsupported("", &path.display().to_string()))?;
        if name.contains('\n') {
            return Err(unsupported("", name));
        }
        return Ok(file_cid(name, &bytes));
    }
    if meta.is_dir() {
        return tree_cid(&path);
    }
    Err(Error::Diag {
        code: "source-pin-unsupported",
        detail: format!("source `{key}` path `{raw}` is neither a file nor a directory"),
    })
}

fn resolve(project_root: &Path, raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() { path.to_path_buf() } else { project_root.join(path) }
}

fn tree_cid(dir: &Path) -> Result<SnapshotId, Error> {
    let mut entries = BTreeMap::new();
    walk(dir, "", &mut entries)?;
    Ok(SnapshotId::from_digest(&sha256_hex(encode(&entries).as_bytes())))
}

fn walk(dir: &Path, prefix: &str, entries: &mut BTreeMap<String, Entry>) -> Result<(), Error> {
    for entry in crate::fs::dir_entries(dir)? {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(unsupported(prefix, &name.to_string_lossy()));
        };
        if name.contains('\n') {
            return Err(unsupported(prefix, name));
        }
        let rel = if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
        if snapshot::ignored(&rel) {
            continue;
        }
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&path)?;
            let Some(target) = target.to_str() else {
                return Err(unsupported(prefix, name));
            };
            let blob = sha256_hex(target.as_bytes());
            entries.insert(rel, Entry::Link { blob });
        } else if meta.is_dir() {
            walk(&path, &rel, entries)?;
        } else {
            let bytes = std::fs::read(&path).map_err(|source| Error::Filesystem {
                op: "read",
                path: path.clone(),
                source,
            })?;
            entries.insert(
                rel,
                Entry::File {
                    exec: is_exec(&meta),
                    blob: sha256_hex(&bytes),
                },
            );
        }
    }
    Ok(())
}

enum Entry {
    File { exec: bool, blob: String },
    Link { blob: String },
}

fn encode(entries: &BTreeMap<String, Entry>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for (path, entry) in entries {
        match entry {
            Entry::File { exec, blob } => {
                let mode = if *exec { "100755" } else { "100644" };
                writeln!(out, "blob {mode} {blob} {path}").expect("String write");
            }
            Entry::Link { blob } => {
                writeln!(out, "link {blob} {path}").expect("String write");
            }
        }
    }
    out
}

fn unsupported(prefix: &str, name: &str) -> Error {
    Error::Diag {
        code: "workspace-path-unsupported",
        detail: format!("path `{prefix}/{name}` is not a snapshot-safe UTF-8 name"),
    }
}

#[cfg(unix)]
fn is_exec(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o100 != 0
}

#[cfg(not(unix))]
fn is_exec(_meta: &std::fs::Metadata) -> bool {
    false
}
