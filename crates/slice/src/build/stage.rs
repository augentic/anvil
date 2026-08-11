//! The RFC-90 D5 artifact stage kernel: [`seed`] mirrors the slice
//! tree at `<attempt_dir>/stage/`, [`Stage::diff`] + [`enforce_grants`]
//! gate the change set, and [`Stage::promote`] commits all-or-none.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::{Error, Result};
use project::adapter::{ArtifactDeclaration, WritableArtifactKind};

use super::gate::malformed_relative_path;

/// An attempt-local artifact stage: the writable mirror root plus the
/// seed-time content manifest its diff compares against.
#[derive(Debug, Clone)]
pub struct Stage {
    /// The stage root, `<attempt_dir>/stage/`.
    root: PathBuf,
    /// Seed-time '/'-separated relative path → content SHA-256 hex.
    manifest: BTreeMap<String, String>,
}

/// One divergence between the stage and its seed manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageChange {
    /// '/'-separated slice-relative path.
    pub path: String,
    /// What happened to the path.
    pub kind: ChangeKind,
}

/// Closed change classification for one staged path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Present in the stage, absent from the seed.
    Added,
    /// Present in both with differing content.
    Modified,
    /// Present in the seed, absent from the stage.
    Deleted,
}

/// Engine-owned top-level slice subtrees excluded from the seed
/// mirror: never target-writable slice intent (`build/` also keeps a
/// re-seed from recursing into stages).
const ENGINE_OWNED: [&str; 4] = ["build", "builds", "decisions", "merge"];

/// Seed the stage: copy the slice tree into `<attempt_dir>/stage/`,
/// recording a manifest of relative path → content SHA-256.
///
/// The slice's engine-owned top-level subtrees (`build/` attempt
/// records, `builds/` `BuildRecord`s, `merge/` gate reports,
/// `decisions/` Decision Records) are excluded — they are not
/// target-writable slice intent. Symlinks are skipped: the mirror
/// carries regular content only.
///
/// # Errors
///
/// Propagates filesystem failures reading the slice tree or writing
/// the mirror.
pub fn seed(attempt_dir: &Path, slice_dir: &Path) -> Result<Stage> {
    let root = attempt_dir.join("stage");
    std::fs::create_dir_all(&root)?;
    let mut manifest = BTreeMap::new();
    copy_tree(slice_dir, &root, "", true, &mut manifest)?;
    Ok(Stage { root, manifest })
}

/// Recursively mirror `source` into `destination`, folding file
/// digests into `manifest` under `prefix`-joined relative paths.
/// `top_level` excludes the [`ENGINE_OWNED`] entries at the root
/// only; symlinks are skipped without following them.
fn copy_tree(
    source: &Path, destination: &Path, prefix: &str, top_level: bool,
    manifest: &mut BTreeMap<String, String>,
) -> Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if top_level && ENGINE_OWNED.contains(&name.as_str()) {
            continue;
        }
        let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let mirrored = destination.join(&name);
            std::fs::create_dir_all(&mirrored)?;
            copy_tree(&path, &mirrored, &relative, false, manifest)?;
        } else if metadata.is_file() {
            let bytes = std::fs::read(&path)?;
            std::fs::write(destination.join(&name), &bytes)?;
            manifest.insert(relative, sha256_hex(&bytes));
        }
    }
    Ok(())
}

impl Stage {
    /// The agent-visible stage root, `<attempt_dir>/stage/`.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Derive the actual staged change set: walk the stage tree and
    /// compare it against the seed manifest. Paths are '/'-separated
    /// relative paths; the result is sorted by path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] keyed on
    /// `target-build-artifact-symlink` when the stage holds a symlink
    /// (a link could smuggle external content past promotion), and
    /// propagates filesystem failures walking the stage.
    pub fn diff(&self) -> Result<Vec<StageChange>> {
        let mut current = BTreeMap::new();
        walk(&self.root, "", &mut current)?;
        let mut changes = Vec::new();
        for (path, digest) in &current {
            match self.manifest.get(path) {
                None => changes.push(StageChange {
                    path: path.clone(),
                    kind: ChangeKind::Added,
                }),
                Some(seeded) if seeded != digest => changes.push(StageChange {
                    path: path.clone(),
                    kind: ChangeKind::Modified,
                }),
                Some(_) => {}
            }
        }
        for path in self.manifest.keys() {
            if !current.contains_key(path) {
                changes.push(StageChange {
                    path: path.clone(),
                    kind: ChangeKind::Deleted,
                });
            }
        }
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(changes)
    }

    /// Promote `changes` from the stage onto the authoritative slice
    /// tree as an engine-owned recoverable transaction (RFC-90 D5).
    ///
    /// Validates the complete diff first, prepares every replacement
    /// as a `<index>.promote.tmp` temporary under the attempt
    /// directory (same filesystem, never the authoritative slice tree
    /// — a crash strands no temporary where validators run), then
    /// commits the set all-or-none, rolling back before any error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] keyed on
    /// `target-build-artifact-promotion-failed` on any validation,
    /// prepare, or commit failure; on a commit failure the already
    /// committed subset is rolled back first.
    pub fn promote(&self, changes: &[StageChange], slice_dir: &Path) -> Result<()> {
        // Validate: read every replacement source and snapshot every
        // original before any mutation.
        let mut replacements: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
        for change in changes {
            if matches!(change.kind, ChangeKind::Added | ChangeKind::Modified) {
                let bytes = std::fs::read(self.root.join(&change.path)).map_err(|err| {
                    promotion_failed(&change.path, "staged source unreadable", &err)
                })?;
                replacements.insert(&change.path, bytes);
            }
        }
        let mut originals: BTreeMap<&str, Option<Vec<u8>>> = BTreeMap::new();
        for change in changes {
            originals.insert(&change.path, snapshot_original(&slice_dir.join(&change.path))?);
        }

        // Prepare: write every replacement as a temporary under the
        // attempt directory and pre-create its destination's parents.
        let staging = self.root.parent().unwrap_or(&self.root);
        let mut temporaries: BTreeMap<&str, PathBuf> = BTreeMap::new();
        for (index, (path, bytes)) in replacements.iter().enumerate() {
            let temporary = staging.join(format!("{index}.promote.tmp"));
            let prepared = slice_dir
                .join(path)
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&temporary, bytes));
            match prepared {
                Ok(()) => {
                    temporaries.insert(path, temporary);
                }
                Err(err) => {
                    remove_temporaries(temporaries.values());
                    return Err(promotion_failed(path, "prepare failed", &err));
                }
            }
        }

        // Commit all-or-none.
        let mut committed: Vec<&str> = Vec::new();
        for change in changes {
            let destination = slice_dir.join(&change.path);
            let result = match change.kind {
                ChangeKind::Added | ChangeKind::Modified => temporaries
                    .get(change.path.as_str())
                    .ok_or_else(|| std::io::Error::other("prepared temporary missing"))
                    .and_then(|temporary| std::fs::rename(temporary, &destination)),
                ChangeKind::Deleted => match std::fs::remove_file(&destination) {
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    other => other,
                },
            };
            match result {
                Ok(()) => committed.push(&change.path),
                Err(err) => {
                    rollback(slice_dir, &committed, &originals);
                    remove_temporaries(temporaries.values());
                    return Err(promotion_failed(&change.path, "commit failed", &err));
                }
            }
        }
        Ok(())
    }
}

/// Snapshot the original bytes of a destination for rollback.
///
/// `Some` for a regular file, `None` when absent (or a directory —
/// nothing restorable; its commit will fail and abort the
/// transaction).
///
/// # Errors
///
/// Returns the promotion failure when an existing regular file cannot
/// be read.
fn snapshot_original(destination: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::metadata(destination) {
        Ok(meta) if meta.is_file() => std::fs::read(destination).map(Some).map_err(|err| {
            promotion_failed(&destination.to_string_lossy(), "original unreadable", &err)
        }),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => {
            Err(promotion_failed(&destination.to_string_lossy(), "original unreadable", &err))
        }
    }
}

/// Best-effort restore of the committed subset: rewrite snapshotted
/// originals, remove destinations that did not exist before.
fn rollback(slice_dir: &Path, committed: &[&str], originals: &BTreeMap<&str, Option<Vec<u8>>>) {
    for path in committed {
        let destination = slice_dir.join(path);
        let restored = match originals.get(path) {
            Some(Some(bytes)) => std::fs::write(&destination, bytes),
            Some(None) | None => match std::fs::remove_file(&destination) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
        };
        if let Err(err) = restored {
            tracing::warn!("promotion rollback failed for `{path}`: {err}");
        }
    }
}

/// Best-effort removal of prepared `.promote.tmp` files.
fn remove_temporaries<'a>(temporaries: impl Iterator<Item = &'a PathBuf>) {
    for temporary in temporaries {
        if let Err(err) = std::fs::remove_file(temporary)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                "promotion temporary cleanup failed for `{}`: {err}",
                temporary.display()
            );
        }
    }
}

/// The transaction's single diagnostic shape.
fn promotion_failed(path: &str, stage: &str, err: &std::io::Error) -> Error {
    Error::Diag {
        code: "target-build-artifact-promotion-failed",
        detail: format!("artifact promotion {stage} for `{path}`: {err}"),
    }
}

/// Gate the staged change set against the target's declared
/// `writable-artifacts[]` grants (RFC-90 D5).
///
/// A `file` grant covers exactly its path; a `tree` grant covers the
/// path itself and every path under `<path>/`. Every change must be
/// covered — even one the phase omitted from `written`. Grant
/// grammar is validated defensively first (relative '/'-separated
/// paths, no `..`, no backslash).
///
/// # Errors
///
/// - [`Error::Diag`] keyed on `target-build-artifact-grant-invalid`
///   when a grant's own path grammar is malformed;
/// - [`Error::Diag`] keyed on `target-build-artifact-scope-violation`
///   naming the first change path outside every grant.
pub fn enforce_grants(changes: &[StageChange], grants: &[ArtifactDeclaration]) -> Result<()> {
    for grant in grants {
        if malformed_relative_path(&grant.path) {
            return Err(Error::Diag {
                code: "target-build-artifact-grant-invalid",
                detail: format!(
                    "writable-artifacts grant `{}` is not a well-formed relative path",
                    grant.path
                ),
            });
        }
    }
    for change in changes {
        let covered = grants.iter().any(|grant| covers(grant, &change.path));
        if !covered {
            return Err(Error::Diag {
                code: "target-build-artifact-scope-violation",
                detail: format!(
                    "staged artifact change `{}` is outside the target's declared \
                     writable-artifacts grants",
                    change.path
                ),
            });
        }
    }
    Ok(())
}

/// Whether one grant covers one change path.
fn covers(grant: &ArtifactDeclaration, path: &str) -> bool {
    let granted = grant.path.trim_end_matches('/');
    match grant.kind {
        WritableArtifactKind::File => path == granted,
        WritableArtifactKind::Tree => {
            path == granted || path.strip_prefix(granted).is_some_and(|rest| rest.starts_with('/'))
        }
    }
}

/// Best-effort removal of `<attempt_dir>/stage/` on a terminal path.
/// Failures are logged, never propagated — a leaked stage is GC
/// territory, not a build failure.
pub fn discard(attempt_dir: &Path) {
    let stage = attempt_dir.join("stage");
    if !stage.exists() {
        return;
    }
    if let Err(err) = std::fs::remove_dir_all(&stage) {
        tracing::warn!("stage discard failed for `{}`: {err}", stage.display());
    }
}

/// Walk `root` recursively, folding file digests into `current` under
/// `prefix`-joined relative paths. A symlink is rejected without
/// being followed — the agent-writable stage admits regular files and
/// directories only.
fn walk(root: &Path, prefix: &str, current: &mut BTreeMap<String, String>) -> Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Diag {
                code: "target-build-artifact-symlink",
                detail: format!(
                    "staged artifact `{relative}` is a symlink; the artifact stage admits \
                     regular files and directories only"
                ),
            });
        }
        if metadata.is_dir() {
            walk(&path, &relative, current)?;
        } else if metadata.is_file() {
            current.insert(relative, sha256_hex(&std::fs::read(&path)?));
        }
    }
    Ok(())
}
