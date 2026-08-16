//! The private-workspace kernel: `prepare` / `capture` / `discard`
//! over the content-addressed snapshot [`Store`]. Wasm-clean — it
//! runs in the engine guest and in native deployments alike.

mod exec;
mod manifest;
mod membership;
mod objects;
mod store;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use error::Error;
pub use exec::{ExecMode, FsExecMode};
pub(crate) use membership::Ignores;
pub use objects::{FsObjects, Objects};
use serde::{Deserialize, Serialize};
pub use store::Store;

use crate::snapshot::{CodePatch, SnapshotId};

/// The caller-authored access manifest for one preparation.
///
/// `writable: false` is a read-only source view — the same
/// preparation, discarded without capture. Read-only artifact roots
/// are granted at the seam (the model lend), not materialized here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Access {
    /// Whether the execution may write the workspace and capture a
    /// result.
    pub writable: bool,
}

/// One prepared private workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Opaque workspace identity — the directory name beneath the
    /// workspaces root.
    pub id: String,
    /// Absolute root of the materialized tree.
    pub root: PathBuf,
    /// The snapshot the workspace was prepared from.
    pub base: SnapshotId,
    /// Whether capture is permitted.
    pub writable: bool,
}

/// Host-local execution record beside each workspace directory —
/// machinery, not workflow state. Lets `capture` and `discard`
/// resolve a workspace from its id alone and keeps the recorded base
/// out of the captured tree.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Meta {
    base: SnapshotId,
    writable: bool,
}

/// Prepare a fresh private workspace from `base`.
///
/// Every call yields a new directory beneath `workspaces`; no two
/// executions share a writable tree. Exclusivity follows from
/// construction — nothing is locked or leased.
///
/// # Errors
///
/// `snapshot-missing` when `base` is not in the store; filesystem
/// failures.
pub async fn prepare(
    store: &Store<impl Objects>, workspaces: &Path, base: &SnapshotId, access: Access,
) -> Result<Workspace, Error> {
    std::fs::create_dir_all(workspaces)?;
    let (id, root) = fresh_dir(workspaces)?;
    store.materialize(base, &root).await?;
    artifacts::atomic::yaml_write(
        &meta_path(workspaces, &id),
        &Meta {
            base: base.clone(),
            writable: access.writable,
        },
    )?;
    Ok(Workspace {
        id,
        root,
        base: base.clone(),
        writable: access.writable,
    })
}

/// Resolve a previously prepared workspace from its id.
///
/// # Errors
///
/// `workspace-id-malformed` for a non-simple id, `workspace-missing`
/// when the directory or its execution record is gone.
pub fn resolve(workspaces: &Path, id: &str) -> Result<Workspace, Error> {
    check_id(id)?;
    let root = workspaces.join(id);
    let meta_path = meta_path(workspaces, id);
    if !root.is_dir() || !meta_path.is_file() {
        return Err(Error::Diag {
            code: "workspace-missing",
            detail: format!("workspace `{id}` does not exist"),
        });
    }
    let meta: Meta = serde_saphyr::from_str(&crate::fs::read_text(&meta_path)?)?;
    Ok(Workspace {
        id: id.to_string(),
        root,
        base: meta.base,
        writable: meta.writable,
    })
}

/// Capture the workspace's result tree as a new snapshot.
///
/// Stores and verifies every object, then derives the touched paths
/// against the recorded base. Creates no commit, branch, or
/// completion fact — the caller records completion only after
/// capture succeeds.
///
/// # Errors
///
/// `workspace-missing` / `workspace-id-malformed` on resolution
/// failures, `workspace-read-only` for a source view.
pub async fn capture(
    store: &Store<impl Objects>, workspaces: &Path, id: &str,
) -> Result<CodePatch, Error> {
    let workspace = resolve(workspaces, id)?;
    if !workspace.writable {
        return Err(Error::Diag {
            code: "workspace-read-only",
            detail: format!("workspace `{id}` is a read-only view; nothing to capture"),
        });
    }
    let result = store.snapshot(&workspace.root).await?;
    let touched = store.manifest(&workspace.base).await?.diff(&store.manifest(&result).await?);
    Ok(CodePatch {
        base: workspace.base,
        result,
        touched,
    })
}

/// Discard a workspace: remove its directory and execution record.
/// Idempotent — a missing workspace is already discarded. Completed
/// result snapshots remain available by digest.
///
/// # Errors
///
/// `workspace-id-malformed` for a non-simple id; filesystem failures
/// other than absence.
pub fn discard(workspaces: &Path, id: &str) -> Result<(), Error> {
    check_id(id)?;
    remove_existing_dir(&workspaces.join(id))?;
    remove_existing_file(&meta_path(workspaces, id))?;
    Ok(())
}

/// Compose same-base, disjoint patches into one result snapshot
/// (RFC-96 D6).
///
/// Every patch must start from `base` and the `touched` sets must be
/// pairwise disjoint — there is no textual merge. Exact result-tree
/// values apply in the given order at the manifest level, so the
/// composed identity is deterministic and deployment-independent;
/// the store gains only the composed manifest object. An empty patch
/// set composes to `base` itself.
///
/// # Errors
///
/// `workspace-compose-base-mismatch` / `workspace-compose-overlap`
/// before any store read; `snapshot-missing` for an absent base or
/// result manifest. Failures mutate nothing.
pub async fn compose(
    store: &Store<impl Objects>, base: &SnapshotId, patches: &[CodePatch],
) -> Result<CodePatch, Error> {
    check_composable(base, patches)?;
    let base_manifest = store.manifest(base).await?;
    let mut composed = base_manifest.clone();
    for patch in patches {
        let result = store.manifest(&patch.result).await?;
        for path in &patch.touched {
            match result.entries.get(path) {
                Some(entry) => composed.entries.insert(path.clone(), entry.clone()),
                None => composed.entries.remove(path),
            };
        }
    }
    let result = store.put_manifest(&composed).await?;
    Ok(CodePatch {
        base: base.clone(),
        result,
        touched: base_manifest.diff(&composed),
    })
}

/// Refuse composition unless every patch starts from `base` and the
/// touched sets are pairwise disjoint.
fn check_composable(base: &SnapshotId, patches: &[CodePatch]) -> Result<(), Error> {
    let mut claimed = std::collections::BTreeSet::new();
    for patch in patches {
        if patch.base != *base {
            return Err(Error::Diag {
                code: "workspace-compose-base-mismatch",
                detail: format!(
                    "patch result `{}` starts from `{}`, not the composition base `{base}`",
                    patch.result, patch.base
                ),
            });
        }
        for path in &patch.touched {
            if !claimed.insert(path.as_str()) {
                return Err(Error::Diag {
                    code: "workspace-compose-overlap",
                    detail: format!("path `{path}` is touched by more than one patch"),
                });
            }
        }
    }
    Ok(())
}

/// Garbage-collect abandoned workspaces.
///
/// Removes every entry whose modification time is older than
/// `cutoff`. Best-effort per entry — a busy or vanished entry is
/// skipped, not fatal. Returns the number of workspaces removed.
///
/// # Errors
///
/// Filesystem failures listing the workspaces root (an absent root is
/// zero, not an error).
pub fn gc(workspaces: &Path, cutoff: SystemTime) -> Result<usize, Error> {
    if !workspaces.is_dir() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in crate::fs::dir_entries(workspaces)? {
        let path = entry.path();
        let stale = std::fs::symlink_metadata(&path)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified < cutoff);
        if !stale {
            continue;
        }
        let is_dir = path.is_dir();
        let gone = if is_dir {
            std::fs::remove_dir_all(&path).is_ok()
        } else {
            std::fs::remove_file(&path).is_ok()
        };
        if gone && is_dir {
            removed += 1;
        }
    }
    Ok(removed)
}

fn meta_path(workspaces: &Path, id: &str) -> PathBuf {
    workspaces.join(format!("{id}.yaml"))
}

/// Reject ids that are not a single plain path component — the id
/// crosses the seam as a string and must never traverse.
fn check_id(id: &str) -> Result<(), Error> {
    let simple = !id.is_empty()
        && id != "."
        && id != ".."
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if simple {
        return Ok(());
    }
    Err(Error::Diag {
        code: "workspace-id-malformed",
        detail: format!("workspace id `{id}` is not a plain directory name"),
    })
}

/// Create a fresh uniquely named workspace directory, retrying on the
/// (astronomically unlikely) name collision.
fn fresh_dir(workspaces: &Path) -> Result<(String, PathBuf), Error> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    loop {
        let seed = format!(
            "{:?}:{}:{}",
            SystemTime::now(),
            process_entropy(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let id = format!("ws-{}", &diagnostics::digest::sha256_hex(seed.as_bytes())[..12]);
        let root = workspaces.join(&id);
        match std::fs::create_dir(&root) {
            Ok(()) => return Ok((id, root)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(Error::Io(err)),
        }
    }
}

/// A per-process random value distinguishing concurrent processes in
/// the workspace-id seed. `RandomState` draws real entropy on every
/// platform, including wasm32 — `std::process::id` does not exist
/// there.
fn process_entropy() -> u64 {
    use std::hash::{BuildHasher as _, RandomState};
    static ENTROPY: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *ENTROPY.get_or_init(|| RandomState::new().hash_one(0_u64))
}

fn remove_existing_dir(path: &Path) -> Result<(), Error> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::Io(err)),
    }
}

fn remove_existing_file(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::Io(err)),
    }
}
