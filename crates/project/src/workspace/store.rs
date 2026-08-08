//! The local content-addressed snapshot store.
//!
//! Writes are atomic and write-once (an existing object is never
//! rewritten); reads verify the digest, so corruption fails typed.

use std::path::{Path, PathBuf};

use error::Error;

use super::manifest::{Entry, Manifest};
use crate::snapshot::{CodePatch, SnapshotId};

/// Path components excluded from every snapshot walk: version-control
/// state and the Emery change tree are never product code (RFC-87 D4).
const IGNORED: [&str; 2] = [".git", ".emery"];

/// Root-level names excluded from every snapshot walk: the plan
/// artifacts living at the repo root (`change.md` + `plan.yaml`, the
/// authored `discovery.md`, and a workspace repo's `registry.yaml`)
/// are change-tree state, not product code — capturing them would let
/// the interim apply rewind live plan state.
const IGNORED_ROOT: [&str; 4] = ["change.md", "discovery.md", "plan.yaml", "registry.yaml"];

/// The local snapshot store rooted at
/// [`Locations::snapshots_root`](crate::handler::Locations::snapshots_root).
#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Open the store at `root`; directories are created lazily on
    /// first write.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Snapshot the complete tree at `dir` into the store and return
    /// its identity. Ignored components (`.git`, `.emery`) are
    /// excluded; empty directories are not tracked.
    ///
    /// # Errors
    ///
    /// Filesystem failures, `workspace-path-unsupported` for non-UTF-8
    /// or newline-bearing names.
    pub fn snapshot(&self, dir: &Path) -> Result<SnapshotId, Error> {
        let mut manifest = Manifest::default();
        // Self-exclusion: a store nested beneath the walked tree (test
        // and lab layouts) must never snapshot its own objects — the
        // walk itself writes them.
        let own_root = std::path::absolute(&self.root).ok();
        self.walk(dir, "", own_root.as_deref(), &mut manifest)?;
        let digest = self.put(manifest.encode().as_bytes())?;
        Ok(SnapshotId::from_digest(&digest))
    }

    /// Materialize snapshot `id` into `dest`, creating files with
    /// their recorded modes and symlinks with their recorded targets.
    /// Every object read is digest-verified.
    ///
    /// # Errors
    ///
    /// `snapshot-missing` for an unknown identity,
    /// `snapshot-object-corrupt` on digest drift, filesystem failures.
    pub fn materialize(&self, id: &SnapshotId, dest: &Path) -> Result<(), Error> {
        let manifest = self.manifest(id)?;
        std::fs::create_dir_all(dest)?;
        for (path, entry) in &manifest.entries {
            self.write_entry(dest, path, entry)?;
        }
        Ok(())
    }

    /// Apply `patch` to the tree at `dir`: rewrite each touched path
    /// from the result snapshot, deleting touched paths the result no
    /// longer carries.
    ///
    /// Only the patch's touched paths are written — everything else in
    /// `dir` (including paths the engine's own deterministic merge just
    /// folded) is left untouched.
    ///
    /// # Errors
    ///
    /// `snapshot-missing` for an unknown result identity, filesystem
    /// failures.
    pub fn apply(&self, patch: &CodePatch, dir: &Path) -> Result<(), Error> {
        let target = self.manifest(&patch.result)?;
        for path in &patch.touched {
            if let Some(entry) = target.entries.get(path) {
                self.write_entry(dir, path, entry)?;
            } else {
                remove_entry(&dir.join(path))?;
                prune_empty_parents(dir, path);
            }
        }
        Ok(())
    }

    /// Write one manifest entry beneath `root`, replacing whatever is
    /// there (a stale file, symlink, or directory).
    fn write_entry(&self, root: &Path, path: &str, entry: &Entry) -> Result<(), Error> {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        remove_entry(&target)?;
        match entry {
            Entry::File { exec, blob } => {
                std::fs::write(&target, self.get(blob)?)?;
                set_exec(&target, *exec)?;
            }
            Entry::Link { blob } => {
                let link_target =
                    String::from_utf8(self.get(blob)?).map_err(|_utf8| Error::Diag {
                        code: "snapshot-object-corrupt",
                        detail: format!("symlink target for `{path}` is not UTF-8"),
                    })?;
                symlink(Path::new(&link_target), &target)?;
            }
        }
        Ok(())
    }

    /// Whether snapshot `id`'s manifest object is present.
    #[must_use]
    pub fn contains(&self, id: &SnapshotId) -> bool {
        self.object_path(id.digest()).is_file()
    }

    /// Read and parse snapshot `id`'s manifest.
    ///
    /// # Errors
    ///
    /// `snapshot-missing` when the manifest object is absent.
    pub(crate) fn manifest(&self, id: &SnapshotId) -> Result<Manifest, Error> {
        if !self.contains(id) {
            return Err(Error::Diag {
                code: "snapshot-missing",
                detail: format!("snapshot `{id}` is not in the store"),
            });
        }
        let bytes = self.get(id.digest())?;
        let text = String::from_utf8(bytes).map_err(|_utf8| Error::Diag {
            code: "snapshot-object-corrupt",
            detail: format!("manifest for `{id}` is not UTF-8"),
        })?;
        Manifest::parse(&text)
    }

    /// Store `bytes` as an object, returning its digest. Write-once:
    /// an existing object is left untouched (equal digest means equal
    /// content).
    fn put(&self, bytes: &[u8]) -> Result<String, Error> {
        let digest = diagnostics::digest::sha256_hex(bytes);
        let path = self.object_path(&digest);
        if !path.is_file() {
            artifacts::atomic::bytes_write(&path, bytes)?;
        }
        Ok(digest)
    }

    /// Read the object named `digest`, verifying its content hashes
    /// back to the name.
    fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        let path = self.object_path(digest);
        let bytes = std::fs::read(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path,
            source,
        })?;
        if diagnostics::digest::sha256_hex(&bytes) != digest {
            return Err(Error::Diag {
                code: "snapshot-object-corrupt",
                detail: format!("object `{digest}` failed digest verification"),
            });
        }
        Ok(bytes)
    }

    fn object_path(&self, digest: &str) -> PathBuf {
        self.root.join("objects").join(&digest[..2]).join(&digest[2..])
    }

    /// Depth-first walk folding `dir` into `manifest`. `prefix` is the
    /// `/`-separated relative path of `dir` (empty at the root);
    /// `own_root` is the store's absolute root, skipped when nested.
    fn walk(
        &self, dir: &Path, prefix: &str, own_root: Option<&Path>, manifest: &mut Manifest,
    ) -> Result<(), Error> {
        for entry in crate::fs::dir_entries(dir)? {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return Err(unsupported(prefix, &name.to_string_lossy()));
            };
            if name.contains('\n') {
                return Err(unsupported(prefix, name));
            }
            if IGNORED.contains(&name) || (prefix.is_empty() && IGNORED_ROOT.contains(&name)) {
                continue;
            }
            let rel = if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
            let path = entry.path();
            if own_root.is_some() && std::path::absolute(&path).ok().as_deref() == own_root {
                continue;
            }
            // `symlink_metadata` so a link to a directory records as a
            // link instead of being followed into a foreign tree.
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                let target = std::fs::read_link(&path)?;
                let Some(target) = target.to_str() else {
                    return Err(unsupported(prefix, name));
                };
                let blob = self.put(target.as_bytes())?;
                manifest.entries.insert(rel, Entry::Link { blob });
            } else if meta.is_dir() {
                self.walk(&path, &rel, own_root, manifest)?;
            } else {
                let bytes = std::fs::read(&path).map_err(|source| Error::Filesystem {
                    op: "read",
                    path: path.clone(),
                    source,
                })?;
                let blob = self.put(&bytes)?;
                manifest.entries.insert(
                    rel,
                    Entry::File {
                        exec: is_exec(&meta),
                        blob,
                    },
                );
            }
        }
        Ok(())
    }
}

/// Remove whatever sits at `path` — file, symlink, or directory —
/// treating absence as already removed.
fn remove_entry(path: &Path) -> Result<(), Error> {
    let removed = match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => Err(err),
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
    };
    removed.map_err(Error::Io)
}

/// Best-effort removal of directories left empty by a deletion, walking
/// `path`'s parents up to (never including) `root`.
fn prune_empty_parents(root: &Path, path: &str) {
    let mut parent = Path::new(path).parent();
    while let Some(rel) = parent {
        if rel.as_os_str().is_empty() || std::fs::remove_dir(root.join(rel)).is_err() {
            return;
        }
        parent = rel.parent();
    }
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

#[cfg(unix)]
fn set_exec(path: &Path, exec: bool) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = if exec { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_exec(_path: &Path, _exec: bool) -> Result<(), Error> {
    Ok(())
}

#[cfg(unix)]
fn symlink(original: &Path, link: &Path) -> Result<(), Error> {
    std::os::unix::fs::symlink(original, link)?;
    Ok(())
}

#[cfg(windows)]
fn symlink(original: &Path, link: &Path) -> Result<(), Error> {
    std::os::windows::fs::symlink_file(original, link)?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn symlink(_original: &Path, _link: &Path) -> Result<(), Error> {
    Err(Error::Diag {
        code: "workspace-path-unsupported",
        detail: "symlinks are unsupported on this platform".to_string(),
    })
}
