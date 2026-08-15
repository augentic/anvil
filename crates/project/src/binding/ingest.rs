//! Stage a locator, snapshot it as a tree CID, intern both-roles reuse.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use error::Error;

use super::https::check as check_https;
use super::locator::{Location, Locator};
use super::meter::Meter;
use super::policy::Policy;
use crate::seam::{self, Workspaces};
use crate::snapshot::SnapshotId;
use crate::workspace::{self, Access, Objects, Store, Workspace};

/// Host-staged tree, or a local filesystem read.
#[derive(Clone, Copy, Debug)]
pub enum Staged<'a> {
    /// Read the locator from disk (path locators only).
    Disk,
    /// Host-staged tree (Git checkout, or an HTTPS document staged as
    /// a one-file tree). `.git` is ignored by snapshot.
    Tree(&'a Path),
}

/// One locator resolved to an exact origin and a store CID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// Exact locator (Git revisions are SHAs).
    pub location: Location,
    /// Tree identity of the staged file-or-tree.
    pub cid: SnapshotId,
    /// Freshness warning (moved branch); ingest still used the recorded SHA.
    pub warning: Option<String>,
}

/// Intern map: canonical locator+path → CID, so target and source reuse one tree.
#[derive(Clone, Debug, Default)]
pub struct Cache {
    by_key: BTreeMap<String, SnapshotId>,
}

impl Cache {
    /// Empty intern map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a previously ingested CID for this locator+path.
    #[must_use]
    pub fn get(&self, location: &Location) -> Option<SnapshotId> {
        self.by_key.get(&location.key()).cloned()
    }

    /// Intern `location` onto `cid` so a later both-roles bind reuses it.
    pub fn insert(&mut self, location: &Location, cid: SnapshotId) {
        self.by_key.insert(location.key(), cid);
    }
}

/// CIDs that stay store GC roots for the change lifetime.
#[must_use]
pub fn roots(pins: &[Resolved]) -> Vec<SnapshotId> {
    pins.iter().map(|pin| pin.cid.clone()).collect()
}

/// Read-only source view over a recorded CID (RFC-87 empty writable scope).
///
/// # Errors
///
/// `snapshot-missing`; filesystem failures.
pub async fn view(
    store: &Store<impl Objects>, workspaces: &Path, cid: &SnapshotId,
) -> Result<Workspace, Error> {
    workspace::prepare(store, workspaces, cid, Access { writable: false }).await
}

/// Ingest session: workspace capability, scratch, change root,
/// intern cache, and budgets.
///
/// Wasm-clean — CID minting runs through the seam's [`Workspaces`]
/// capability, origin I/O through [`crate::seam::Trees`] in
/// [`super::fetch_locator`].
#[derive(Debug)]
pub struct Session<'a, W: Workspaces> {
    /// Workspace capability the CID is minted through.
    pub workspaces: &'a W,
    /// Disposable staging directory (one-file trees, copies).
    pub scratch: &'a Path,
    /// Change home; relative path locators join this.
    pub change_root: &'a Path,
    /// Both-roles intern map for this wave bind.
    pub cache: &'a mut Cache,
    /// Compiled D9 limits.
    pub policy: &'a Policy,
    /// Running consumption against [`Self::policy`].
    pub meter: &'a mut Meter,
}

impl<W: Workspaces> Session<'_, W> {
    /// Resolve `location` once, snapshot the staged tree, and intern the CID.
    ///
    /// A recorded CID present in the store is returned without rereading the
    /// origin. A file becomes a one-file tree so every location-backed source
    /// has the same read-only root shape.
    ///
    /// # Errors
    ///
    /// Budget exhaustion, missing paths, escaping symlinks, HTTPS gate
    /// failures, snapshot failures.
    pub async fn ingest(
        &mut self, location: &Location, staged: Staged<'_>, recorded: Option<&SnapshotId>,
        warning: Option<String>,
    ) -> Result<Resolved, Error> {
        self.meter.binding(self.policy)?;
        if let Some(cid) = recorded
            && self.contains(cid).await?
        {
            self.cache.insert(location, cid.clone());
            return Ok(Resolved {
                location: location.clone(),
                cid: cid.clone(),
                warning,
            });
        }
        if let Some(cid) = self.cache.get(location)
            && self.contains(&cid).await?
        {
            return Ok(Resolved {
                location: location.clone(),
                cid,
                warning,
            });
        }
        if let Locator::Https(url) = &location.locator {
            check_https(url)?;
            if location.path != "." {
                return Err(Error::Diag {
                    code: "locator-malformed",
                    detail: "HTTPS locators do not take a path selector".into(),
                });
            }
        }
        let root = self.stage(location, staged)?;
        refuse_escapes(&root)?;
        self.meter.tree(self.policy)?;
        self.charge_tree(&root)?;
        let cid = self
            .workspaces
            .snapshot(root.display().to_string())
            .await
            .map_err(|err| snapshot_failure(&err))?;
        self.cache.insert(location, cid.clone());
        Ok(Resolved {
            location: location.clone(),
            cid,
            warning,
        })
    }

    /// Whether the snapshot store already holds `cid`.
    async fn contains(&self, cid: &SnapshotId) -> Result<bool, Error> {
        self.workspaces.contains(cid.clone()).await.map_err(|err| snapshot_failure(&err))
    }

    fn stage(&self, location: &Location, staged: Staged<'_>) -> Result<PathBuf, Error> {
        match staged {
            Staged::Disk => self.stage_disk(location),
            Staged::Tree(root) => self.wrap(select(root, &location.path)?),
        }
    }

    fn stage_disk(&self, location: &Location) -> Result<PathBuf, Error> {
        let Locator::Path(path) = &location.locator else {
            return Err(Error::Diag {
                code: "locator-malformed",
                detail: "disk ingest requires a path locator".into(),
            });
        };
        let base = if path.is_absolute() { path.clone() } else { self.change_root.join(path) };
        if !base.exists() {
            return Err(Error::Diag {
                code: "locator-path-missing",
                detail: format!("locator path `{}` does not exist", base.display()),
            });
        }
        self.wrap(select(&base, &location.path)?)
    }

    /// A file becomes a one-file tree under scratch so every CID is a tree.
    fn wrap(&self, selected: PathBuf) -> Result<PathBuf, Error> {
        let meta = std::fs::symlink_metadata(&selected)?;
        if meta.is_dir() {
            return Ok(selected);
        }
        let name = selected.file_name().and_then(|n| n.to_str()).ok_or_else(|| Error::Diag {
            code: "locator-malformed",
            detail: format!("file locator `{}` has no UTF-8 name", selected.display()),
        })?;
        let dir = self.scratch.join(unique("file"));
        std::fs::create_dir_all(&dir)?;
        std::fs::copy(&selected, dir.join(name))?;
        Ok(dir)
    }

    fn charge_tree(&mut self, root: &Path) -> Result<(), Error> {
        let mut total = 0_u64;
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in crate::fs::dir_entries(&dir)? {
                let path = entry.path();
                let meta = std::fs::symlink_metadata(&path)?;
                if meta.is_dir() {
                    pending.push(path);
                } else if meta.is_file() {
                    total = total.saturating_add(meta.len());
                }
            }
        }
        self.meter.bytes(total, self.policy)
    }
}

fn select(root: &Path, selector: &str) -> Result<PathBuf, Error> {
    if selector == "." {
        return Ok(root.to_path_buf());
    }
    let rel = Path::new(selector);
    if rel.is_absolute() {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: "path selector must be relative".into(),
        });
    }
    let joined = root.join(rel);
    let normalized = normalize(&joined);
    let root_n = normalize(root);
    if !starts_with(&normalized, &root_n) {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: format!("path selector `{selector}` escapes the locator root"),
        });
    }
    if !joined.exists() {
        return Err(Error::Diag {
            code: "locator-path-missing",
            detail: format!("path selector `{selector}` does not exist"),
        });
    }
    Ok(joined)
}

fn refuse_escapes(root: &Path) -> Result<(), Error> {
    let root_n = normalize(root);
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in crate::fs::dir_entries(&dir)? {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                let target = std::fs::read_link(&path)?;
                if target.is_absolute() {
                    return Err(Error::Diag {
                        code: "locator-symlink-escape",
                        detail: format!(
                            "symlink `{}` has an absolute target",
                            path.strip_prefix(root).unwrap_or(&path).display()
                        ),
                    });
                }
                let resolved = normalize(&path.parent().unwrap_or(&path).join(target));
                if !starts_with(&resolved, &root_n) {
                    return Err(Error::Diag {
                        code: "locator-symlink-escape",
                        detail: format!(
                            "symlink `{}` escapes the ingested tree",
                            path.strip_prefix(root).unwrap_or(&path).display()
                        ),
                    });
                }
            } else if meta.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}

/// A snapshot-capability failure during ingest — the seam error keeps
/// its full detail; the code stays one stable diagnostic.
fn snapshot_failure(err: &seam::Error) -> Error {
    Error::Diag {
        code: "binding-snapshot-failed",
        detail: format!("snapshotting the staged tree failed: {err}"),
    }
}

fn unique(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("{prefix}-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return PathBuf::from("..");
                }
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn starts_with(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}
