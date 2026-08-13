//! The content-addressed snapshot store over the [`Objects`] and
//! [`ExecBits`] seams. The kernel owns hashing: objects are named by
//! the SHA-256 it computed and reads verify the digest.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use error::Error;

use super::exec::{ExecBits, FsExecBits};
use super::manifest::{Entry, Manifest};
use super::objects::{FsObjects, Objects};
use crate::snapshot::{CodePatch, SnapshotId};

/// Path components excluded from every snapshot walk: version-control
/// state and the Emery change tree are never product code (RFC-87 D4).
/// Kernel excludes win over any `.gitignore` negation (RFC-105 D1).
pub const IGNORED: [&str; 2] = [".git", ".emery"];

/// Root-level names excluded from every snapshot walk: the plan
/// artifacts living at the repo root (`change.md` + `plan.yaml` and
/// the authored `discovery.md`) are change-tree state, not product
/// code — capturing them would let the interim apply rewind live plan
/// state.
pub const IGNORED_ROOT: [&str; 3] = ["change.md", "discovery.md", "plan.yaml"];

/// The snapshot store: tree walks and manifests in the kernel, object
/// bytes behind [`Objects`], exec bits behind [`ExecBits`].
#[derive(Clone, Debug)]
pub struct Store<O: Objects> {
    objects: O,
    exec: Arc<dyn ExecBits>,
    /// Self-exclusion root for nested filesystem stores (test and lab
    /// layouts): the walk must never snapshot its own objects.
    exclude: Option<PathBuf>,
}

impl Store<FsObjects> {
    /// Open a filesystem-backed store at `root` (native deployments);
    /// directories are created lazily on first write.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let objects = FsObjects::new(root);
        let exclude = Some(objects.root().to_path_buf());
        Self {
            objects,
            exec: Arc::new(FsExecBits),
            exclude,
        }
    }
}

impl<O: Objects> Store<O> {
    /// Compose a store over explicit seams (the in-guest deployment:
    /// blobstore-backed objects, capability-backed exec bits). No
    /// self-exclusion — the object store is not beneath any walked
    /// tree.
    #[must_use]
    pub fn over(objects: O, exec: impl ExecBits + 'static) -> Self {
        Self {
            objects,
            exec: Arc::new(exec),
            exclude: None,
        }
    }

    /// Snapshot the complete tree at `dir` into the store and return
    /// its identity. Membership is kernel excludes (`.git`, `.emery`,
    /// the root plan files) plus the tree's own `.gitignore` rules
    /// (RFC-105); empty directories are not tracked.
    ///
    /// # Errors
    ///
    /// Filesystem failures, `workspace-path-unsupported` for non-UTF-8
    /// or newline-bearing names.
    pub async fn snapshot(&self, dir: &Path) -> Result<SnapshotId, Error> {
        let mut manifest = Manifest::default();
        let exec = self.exec.read(dir)?;
        let own_root = self.exclude.as_deref().and_then(|root| std::path::absolute(root).ok());
        self.walk(dir, own_root.as_deref(), &exec, &mut manifest).await?;
        let digest = self.put(manifest.encode().as_bytes()).await?;
        Ok(SnapshotId::from_digest(&digest))
    }

    /// [`Self::snapshot`] over a path that may be a single file: a
    /// file snapshots as a one-file tree named by its file name
    /// (mode `100644`, matching the plan-pin `file_cid` encoding); a
    /// directory delegates to [`Self::snapshot`].
    ///
    /// # Errors
    ///
    /// As [`Self::snapshot`], plus `workspace-path-unsupported` for a
    /// non-UTF-8 or newline-bearing file name.
    pub async fn snapshot_path(&self, path: &Path) -> Result<SnapshotId, Error> {
        let meta = std::fs::symlink_metadata(path).map_err(|source| Error::Filesystem {
            op: "stat",
            path: path.to_path_buf(),
            source,
        })?;
        if meta.is_dir() {
            return self.snapshot(path).await;
        }
        let name = path.file_name().and_then(|name| name.to_str()).filter(|n| !n.contains('\n'));
        let Some(name) = name else {
            return Err(Error::Diag {
                code: "workspace-path-unsupported",
                detail: format!("path `{}` is not a snapshot-safe UTF-8 name", path.display()),
            });
        };
        let bytes = std::fs::read(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        let blob = self.put(&bytes).await?;
        let mut manifest = Manifest::default();
        manifest.entries.insert(name.to_string(), Entry::File { exec: false, blob });
        let digest = self.put(manifest.encode().as_bytes()).await?;
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
    pub async fn materialize(&self, id: &SnapshotId, dest: &Path) -> Result<(), Error> {
        let manifest = self.manifest(id).await?;
        std::fs::create_dir_all(dest)?;
        let mut modes = ModeSets::default();
        for (path, entry) in &manifest.entries {
            self.write_entry(dest, path, entry).await?;
            modes.record(path, entry);
        }
        self.exec.apply(dest, &modes.exec, &modes.plain)
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
    pub async fn apply(&self, patch: &CodePatch, dir: &Path) -> Result<(), Error> {
        let target = self.manifest(&patch.result).await?;
        let mut modes = ModeSets::default();
        for path in &patch.touched {
            if let Some(entry) = target.entries.get(path) {
                self.write_entry(dir, path, entry).await?;
                modes.record(path, entry);
            } else {
                remove_entry(&dir.join(path))?;
                prune_empty_parents(dir, path);
            }
        }
        self.exec.apply(dir, &modes.exec, &modes.plain)
    }

    /// Write one manifest entry beneath `root`, replacing whatever is
    /// there (a stale file, symlink, or directory). Exec bits are
    /// applied in bulk by the caller.
    async fn write_entry(&self, root: &Path, path: &str, entry: &Entry) -> Result<(), Error> {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        remove_entry(&target)?;
        match entry {
            Entry::File { blob, .. } => {
                self.copy_file(blob, &target).await?;
            }
            Entry::Link { blob } => {
                let link_target =
                    String::from_utf8(self.get(blob).await?).map_err(|_utf8| Error::Diag {
                        code: "snapshot-object-corrupt",
                        detail: format!("symlink target for `{path}` is not UTF-8"),
                    })?;
                symlink(Path::new(&link_target), &target)?;
            }
        }
        Ok(())
    }

    /// Whether snapshot `id`'s manifest object is present.
    pub async fn contains(&self, id: &SnapshotId) -> bool {
        self.objects.has(id.digest()).await
    }

    /// Sweep the objects reachable from `dead` roots but not from
    /// `live` roots — the change-scoped collection `plan archive`
    /// runs once a plan's pins stop being GC roots (RFC-88 D2).
    ///
    /// Roots absent from the store are skipped (already collected).
    /// Returns the number of objects deleted.
    ///
    /// # Errors
    ///
    /// Manifest read/parse failures on present roots; deletion
    /// failures other than absence.
    pub async fn sweep(&self, dead: &[SnapshotId], live: &[SnapshotId]) -> Result<usize, Error> {
        let keep = self.reachable(live).await?;
        let mut removed = 0;
        for digest in self.reachable(dead).await? {
            if keep.contains(&digest) {
                continue;
            }
            self.objects.delete(&digest).await?;
            removed += 1;
        }
        Ok(removed)
    }

    /// Every object digest reachable from `roots`: each present root's
    /// manifest object plus all blobs its entries name. Absent roots
    /// contribute nothing.
    async fn reachable(&self, roots: &[SnapshotId]) -> Result<BTreeSet<String>, Error> {
        let mut reached = BTreeSet::new();
        for root in roots {
            if !self.contains(root).await {
                continue;
            }
            if !reached.insert(root.digest().to_string()) {
                continue;
            }
            let manifest = self.manifest(root).await?;
            for entry in manifest.entries.values() {
                let (Entry::File { blob, .. } | Entry::Link { blob }) = entry;
                reached.insert(blob.clone());
            }
        }
        Ok(reached)
    }

    /// Read and parse snapshot `id`'s manifest.
    ///
    /// # Errors
    ///
    /// `snapshot-missing` when the manifest object is absent.
    pub(crate) async fn manifest(&self, id: &SnapshotId) -> Result<Manifest, Error> {
        if !self.contains(id).await {
            return Err(Error::Diag {
                code: "snapshot-missing",
                detail: format!("snapshot `{id}` is not in the store"),
            });
        }
        let bytes = self.get(id.digest()).await?;
        let text = String::from_utf8(bytes).map_err(|_utf8| Error::Diag {
            code: "snapshot-object-corrupt",
            detail: format!("manifest for `{id}` is not UTF-8"),
        })?;
        Manifest::parse(&text)
    }

    /// Store `bytes` as an object, returning its digest. Write-once:
    /// an existing object is left untouched (equal digest means equal
    /// content).
    async fn put(&self, bytes: &[u8]) -> Result<String, Error> {
        let digest = diagnostics::digest::sha256_hex(bytes);
        self.objects.put(&digest, bytes).await?;
        Ok(digest)
    }

    /// Stream-hash `path` and store it as an object, returning its
    /// digest. Same identity as [`Self::put`] of the file's bytes.
    async fn put_file(&self, path: &Path) -> Result<String, Error> {
        let digest = hash_path(path)?;
        self.objects.put_file(&digest, path).await?;
        Ok(digest)
    }

    /// Read the object named `digest`, verifying its content hashes
    /// back to the name.
    async fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        let bytes = self.objects.get(digest).await?;
        if diagnostics::digest::sha256_hex(&bytes) != digest {
            return Err(corrupt(digest));
        }
        Ok(bytes)
    }

    /// Stream object `digest` into `dest` and verify the written
    /// bytes hash back to the name.
    async fn copy_file(&self, digest: &str, dest: &Path) -> Result<(), Error> {
        self.objects.copy_file(digest, dest).await?;
        if hash_path(dest)? != digest {
            drop(std::fs::remove_file(dest));
            return Err(corrupt(digest));
        }
        Ok(())
    }

    /// Depth-first walk folding `root` into `manifest`, driven by an
    /// explicit directory stack (awaiting inside recursion would need
    /// boxed futures). `own_root` is a nested filesystem store's
    /// absolute root, skipped when present; `exec` is the tree's
    /// bulk-read exec set. Each pending directory carries its
    /// gitignore stack (RFC-105); an ignored directory is neither
    /// hashed nor descended. Entries are keyed by relative path, so
    /// visit order does not affect snapshot identity.
    async fn walk(
        &self, root: &Path, own_root: Option<&Path>, exec: &BTreeSet<String>,
        manifest: &mut Manifest,
    ) -> Result<(), Error> {
        let mut pending = vec![(root.to_path_buf(), String::new(), super::Ignores::default())];
        while let Some((dir, prefix, ignores)) = pending.pop() {
            let ignores = ignores.descend(&dir);
            for entry in crate::fs::dir_entries(&dir)? {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    return Err(unsupported(&prefix, &name.to_string_lossy()));
                };
                if name.contains('\n') {
                    return Err(unsupported(&prefix, name));
                }
                if IGNORED.contains(&name) || (prefix.is_empty() && IGNORED_ROOT.contains(&name)) {
                    continue;
                }
                let rel =
                    if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
                let path = entry.path();
                if own_root.is_some() && std::path::absolute(&path).ok().as_deref() == own_root {
                    continue;
                }
                // `symlink_metadata` so a link to a directory records as a
                // link instead of being followed into a foreign tree.
                let meta = std::fs::symlink_metadata(&path)?;
                if ignores.excluded(&path, meta.is_dir()) {
                    continue;
                }
                if meta.file_type().is_symlink() {
                    let target = std::fs::read_link(&path)?;
                    let Some(target) = target.to_str() else {
                        return Err(unsupported(&prefix, name));
                    };
                    // Snapshots carry relative links only: an absolute
                    // target cannot be re-created inside a sandboxed
                    // guest, so the refusal is symmetric and early.
                    if Path::new(target).is_absolute() {
                        return Err(Error::Diag {
                            code: "workspace-path-unsupported",
                            detail: format!(
                                "symlink `{rel}` has an absolute target; snapshots carry relative \
                                 links only"
                            ),
                        });
                    }
                    let blob = self.put(target.as_bytes()).await?;
                    manifest.entries.insert(rel, Entry::Link { blob });
                } else if meta.is_dir() {
                    pending.push((path, rel, ignores.clone()));
                } else {
                    let blob = self.put_file(&path).await?;
                    let exec = exec.contains(&rel);
                    manifest.entries.insert(rel, Entry::File { exec, blob });
                }
            }
        }
        Ok(())
    }
}

/// The exec/plain path sets one materialization accumulates for the
/// single bulk [`ExecBits::apply`] call.
#[derive(Default)]
struct ModeSets {
    exec: Vec<String>,
    plain: Vec<String>,
}

impl ModeSets {
    fn record(&mut self, path: &str, entry: &Entry) {
        if let Entry::File { exec, .. } = entry {
            if *exec {
                self.exec.push(path.to_string());
            } else {
                self.plain.push(path.to_string());
            }
        }
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

fn hash_path(path: &Path) -> Result<String, Error> {
    diagnostics::digest::sha256_path(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })
}

fn corrupt(digest: &str) -> Error {
    Error::Diag {
        code: "snapshot-object-corrupt",
        detail: format!("object `{digest}` failed digest verification"),
    }
}

fn unsupported(prefix: &str, name: &str) -> Error {
    Error::Diag {
        code: "workspace-path-unsupported",
        detail: format!("path `{prefix}/{name}` is not a snapshot-safe UTF-8 name"),
    }
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

/// `std::os::wasi::fs::symlink_path` is unstable, so symlink creation
/// resolves the longest-prefix preopen (`.`, the workspaces mount, …)
/// and calls `symlink-at` on its descriptor.
#[cfg(target_os = "wasi")]
fn symlink(original: &Path, link: &Path) -> Result<(), Error> {
    let (original, link) = match (original.to_str(), link.to_str()) {
        (Some(original), Some(link)) => (original, link),
        _ => return Err(unsupported("", &link.to_string_lossy())),
    };
    let mut best: Option<(wasip2::filesystem::types::Descriptor, String)> = None;
    let mut best_len = 0;
    for (descriptor, name) in wasip2::filesystem::preopens::get_directories() {
        let rest = if name == "." {
            (!link.starts_with('/'))
                .then(|| link.strip_prefix("./").unwrap_or(link))
                .map(str::to_string)
        } else {
            link.strip_prefix(&name).and_then(|rest| rest.strip_prefix('/')).map(str::to_string)
        };
        if let Some(rest) = rest
            && name.len() >= best_len
        {
            best_len = name.len();
            best = Some((descriptor, rest));
        }
    }
    let Some((descriptor, rest)) = best else {
        return Err(Error::Io(std::io::Error::other(format!(
            "no preopen reaches `{link}` for symlink creation"
        ))));
    };
    descriptor.symlink_at(original, &rest).map_err(|code| {
        Error::Io(std::io::Error::other(format!("symlink `{link}` -> `{original}`: {code}")))
    })
}

#[cfg(not(any(unix, windows, target_os = "wasi")))]
fn symlink(_original: &Path, _link: &Path) -> Result<(), Error> {
    Err(Error::Diag {
        code: "workspace-path-unsupported",
        detail: "symlinks are unsupported on this platform".to_string(),
    })
}
