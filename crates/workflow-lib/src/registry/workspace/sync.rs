//! Peer materialisation: turn registry entries into `workspace/<name>/`
//! symlinks (local URLs) or Git worktrees of a persistent out-of-tree
//! mirror (remote URLs).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use specify_error::Error;

use super::bootstrap::{self, greenfield_init};
use super::git::{self, git_output_ok};
use super::mirror::mirror_adapters;
use super::slot_problem::inspect_at;
use super::{contracts_base, registry_symlink_target, workspace_base, workspace_slot_path};
use crate::cmd;
use crate::config::{Layout, ProjectConfig};
use crate::registry::catalog::{Registry, RegistryProject};
use crate::registry::gitignore::ensure_gitignore_entries;
use crate::registry::topology::{TopologyLock, TopologyProject};

/// Materialise `workspace/<name>/` for selected registry entries.
///
/// Callers must pass projects returned by
/// [`crate::registry::Registry::select`] so unknown selectors fail before
/// this function performs filesystem, Git, or forge work.
///
/// # Errors
///
/// Aggregates per-project sync failures under the `workspace-sync-failed`
/// diagnostic; refuses materialisation when `workspace/` is itself
/// a symlink or otherwise invalid.
pub fn sync_projects(project_dir: &Path, projects: &[&RegistryProject]) -> Result<(), Error> {
    ensure_gitignore_entries(project_dir)?;

    let base = prepare_workspace_base(project_dir)?;

    let mut errors: Vec<String> = Vec::new();
    for project in projects {
        let result = workspace_slot_path(&base, &project.name).and_then(|dest| {
            if let Some(problem) = inspect_at(project_dir, project, &dest) {
                return Err(Error::Diag {
                    code: "workspace-slot-mismatch",
                    detail: problem.message().to_string(),
                });
            }
            if project.is_local() {
                materialise_symlink(project_dir, &project.url, &dest)
            } else {
                materialise_git_remote(&project.url, &dest, project.adapter.as_deref(), project_dir)
            }
        });
        if let Err(err) = result {
            errors.push(format!("{}: {err}", project.name));
        }
    }

    // Slot adapter provisioning: provision each synced slot's manifest cache with the
    // workspace's adapter set so slot-side resolution stays
    // project-local. Local symlink slots are mirrored too — the write
    // lands only under the peer's out-of-tree per-project cache.
    for project in projects {
        let Ok(slot) = workspace_slot_path(&base, &project.name) else {
            continue;
        };
        if !slot.join(".specify").is_dir() || resolves_to_workspace(project_dir, &slot) {
            continue;
        }
        if let Err(err) = mirror_adapters(project_dir, &slot) {
            errors.push(format!("{} (adapters): {err}", project.name));
        }
    }

    let central_contracts = contracts_base(project_dir);
    if central_contracts.is_dir() {
        for project in projects {
            if project.is_local() {
                continue;
            }
            let Ok(slot) = workspace_slot_path(&base, &project.name) else {
                continue;
            };
            if !slot.is_dir() {
                continue;
            }
            if !slot.join(".specify").is_dir() {
                continue;
            }
            let dest_contracts = slot.join("contracts");
            if let Err(err) = distribute_contracts(&central_contracts, &dest_contracts) {
                errors.push(format!("{} (contracts): {err}", project.name));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Diag {
            code: "workspace-sync-failed",
            detail: format!(
                "workspace sync failed for {} project(s):\n{}",
                errors.len(),
                errors.join("\n")
            ),
        })
    }
}

/// Regenerate `.specify/topology.lock` from the materialised workspace
/// slots.
///
/// Projects every registry member's authored intent (target adapter,
/// description) plus its deterministic baseline identity (`surface[]`
/// from `.specify/specs/`, `recent[]` from the journal ledger) into the
/// committed cache, in registry order. Slots without a readable
/// `project.yaml` yet (a
/// remote not materialised in this selective sync) are skipped — a full
/// `workspace sync` materialises every slot and so produces a complete
/// cache. Write-if-changed: an up-to-date lock is left untouched so the
/// committed bytes stay stable.
///
/// # Errors
///
/// Surfaces target-resolution errors and a malformed/too-new existing
/// lock; an absent slot `project.yaml` is skipped, not an error.
pub fn regenerate_topology_lock(project_dir: &Path, registry: &Registry) -> Result<(), Error> {
    let base = workspace_base(project_dir);
    let mut projects: Vec<TopologyProject> = Vec::new();
    for project in &registry.projects {
        let slot = workspace_slot_path(&base, &project.name)?;
        if !slot.join(".specify").join("project.yaml").exists() {
            continue;
        }
        let config = ProjectConfig::load(&slot)?;
        projects.push(TopologyProject::resolve(&project.name, &config, &slot)?);
    }

    let path = Layout::new(project_dir).topology_lock_path();
    let fresh = TopologyLock::from_projects(projects);
    if TopologyLock::load(&path)?.as_ref() != Some(&fresh) {
        fresh.save(&path)?;
    }
    Ok(())
}

/// A `url: .` registry entry symlinks its slot to the workspace
/// itself; mirroring there would copy the manifest cache over itself.
fn resolves_to_workspace(workspace_dir: &Path, slot: &Path) -> bool {
    match (std::fs::canonicalize(workspace_dir), std::fs::canonicalize(slot)) {
        (Ok(workspace), Ok(slot)) => workspace == slot,
        _ => false,
    }
}

fn prepare_workspace_base(project_dir: &Path) -> Result<PathBuf, Error> {
    let specify_dir = project_dir.join(".specify");
    reject_symlinked_directory(&specify_dir, ".specify/")?;
    std::fs::create_dir_all(&specify_dir).map_err(Error::Io)?;

    let base = workspace_base(project_dir);
    reject_symlinked_directory(&base, "workspace/")?;
    std::fs::create_dir_all(&base).map_err(Error::Io)?;
    reject_symlinked_directory(&base, "workspace/")?;

    Ok(base)
}

fn reject_symlinked_directory(path: &Path, label: &str) -> Result<(), Error> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(Error::Diag {
            code: "workspace-specify-dir-symlink",
            detail: format!(
                "{label} is a symlink; refusing to materialise workspace slots through it"
            ),
        }),
        Ok(meta) if !meta.is_dir() => Err(Error::Diag {
            code: "workspace-specify-dir-not-directory",
            detail: format!(
                "{label} exists but is not a directory; remove it before running workspace sync"
            ),
        }),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::Io(err)),
    }
}

pub(super) fn materialise_symlink(project_dir: &Path, url: &str, dest: &Path) -> Result<(), Error> {
    let target = registry_symlink_target(project_dir, url)?;

    match std::fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_symlink() => match std::fs::canonicalize(dest) {
            Ok(resolved) if resolved == target => return Ok(()),
            Ok(resolved) => {
                return Err(Error::Diag {
                    code: "workspace-slot-symlink-target-mismatch",
                    detail: format!(
                        "workspace/{} already exists as a symlink to {}; expected {} from registry url `{url}`",
                        dest.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                        resolved.display(),
                        target.display()
                    ),
                });
            }
            Err(err) => {
                return Err(Error::Diag {
                    code: "workspace-slot-symlink-broken",
                    detail: format!(
                        "workspace/{} already exists as a broken symlink; expected {} from registry url `{url}` ({err})",
                        dest.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                        target.display()
                    ),
                });
            }
        },
        Ok(_) => {
            return Err(Error::Diag {
                code: "workspace-slot-not-symlink",
                detail: format!(
                    "workspace/{} already exists and is not a symlink; remove it before re-syncing",
                    dest.file_name().and_then(|s| s.to_str()).unwrap_or("?")
                ),
            });
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::Io(e)),
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    symlink(&target, dest)?;
    Ok(())
}

fn symlink(target: &Path, link: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(Error::Io)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).map_err(Error::Io)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(Error::Diag {
            code: "workspace-symlink-unsupported",
            detail: "platform does not support symlinks for `specify workspace sync`".to_string(),
        })
    }
}

pub(super) fn materialise_git_remote(
    url: &str, dest: &Path, adapter: Option<&str>, initiating_project_dir: &Path,
) -> Result<(), Error> {
    match std::fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_symlink() => Err(Error::Diag {
            code: "workspace-remote-slot-is-symlink",
            detail: format!(
                "`{}` is a symlink, but registry url `{url}` is remote-backed; remove the slot before re-syncing",
                dest.display()
            ),
        }),
        Ok(meta) if meta.is_dir() => {
            if !dest.join(".git").exists() {
                return Err(Error::Diag {
                    code: "workspace-remote-slot-not-git-clone",
                    detail: format!(
                        "`{}` exists but is not a git worktree (no `.git`); remove it or pick another registry name",
                        dest.display()
                    ),
                });
            }
            ensure_origin_matches(dest, url)?;
            if dest.join(".specify").join("project.yaml").exists() {
                // A failed fetch leaves a stale registry worktree; surface it
                // rather than masking it with `.or(Ok(()))`. The fetch lands
                // in the shared mirror's `refs/remotes/origin/*`.
                git::run(dest, &["fetch", "origin"], &format!("git fetch in {}", dest.display()))
            } else {
                greenfield_init(dest, require_seed(adapter, dest)?, initiating_project_dir, true)
            }
        }
        Ok(_) => Err(Error::Diag {
            code: "workspace-slot-not-directory",
            detail: format!(
                "`{}` exists but is not a directory; remove it before re-syncing",
                dest.display()
            ),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(Error::Io)?;
            }

            match ensure_mirror(url)? {
                Some(mirror) => {
                    add_worktree(&mirror, dest)?;
                    ensure_origin_matches(dest, url)?;
                    Ok(())
                }
                None => bootstrap::bootstrap(
                    url,
                    dest,
                    require_seed(adapter, dest)?,
                    initiating_project_dir,
                ),
            }
        }
        Err(err) => Err(Error::Io(err)),
    }
}

/// Ensure a persistent bare mirror exists for `url`, fetching the latest
/// objects when it already does.
///
/// The mirror lives out-of-tree (keyed by a digest of `url`) so a peer's
/// object store is shared across changes and fresh checkouts. Returns
/// `None` when the remote is unreachable or empty — the caller bootstraps
/// a greenfield slot offline in that case, matching the prior
/// clone-then-fallback behaviour.
fn ensure_mirror(url: &str) -> Result<Option<PathBuf>, Error> {
    let mirror = specify_schema::cache::mirror_dir(url);

    if mirror.join("HEAD").exists() {
        git::run(
            &mirror,
            &["fetch", "origin"],
            &format!("git fetch in mirror {}", mirror.display()),
        )?;
        return Ok(Some(mirror));
    }

    if let Some(parent) = mirror.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let clone = cmd::git(
        &cmd::real_cmd,
        None,
        [OsStr::new("clone"), OsStr::new("--bare"), OsStr::new(url), mirror.as_os_str()],
    )
    .map_err(|e| Error::Diag {
        code: "workspace-git-clone-spawn-failed",
        detail: format!(
            "failed to spawn `git clone` for registry url `{url}`: {e} (is `git` installed?)"
        ),
    })?;

    if !clone.status.success() {
        // Unreachable or empty remote: drop the partial mirror so a later
        // reachable sync re-clones, and signal greenfield bootstrap.
        drop(std::fs::remove_dir_all(&mirror));
        return Ok(None);
    }

    // A bare clone copies `refs/heads/*` but no remote-tracking refs.
    // Configure the standard refspec and fetch so worktrees resolve
    // `origin/HEAD` and `origin/<branch>` the way branch preparation expects.
    git::run(
        &mirror,
        &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"],
        "git config remote.origin.fetch",
    )?;
    git::run(&mirror, &["fetch", "origin"], &format!("git fetch in mirror {}", mirror.display()))?;

    Ok(Some(mirror))
}

/// Add a worktree at `dest` from the persistent `mirror`, first pruning
/// any stale registration left by a previous (gitignored, since-removed)
/// checkout of the same slot.
fn add_worktree(mirror: &Path, dest: &Path) -> Result<(), Error> {
    git::run(mirror, &["worktree", "prune"], "git worktree prune")?;
    let dest_str = dest.to_string_lossy();
    git::run(
        mirror,
        &["worktree", "add", "--detach", "--force", &dest_str],
        &format!("git worktree add {}", dest.display()),
    )
}

/// A greenfield scaffold needs an adapter; the registry `adapter` is an
/// optional seed, so error clearly when a brand-new slot must be
/// bootstrapped but no seed was declared.
fn require_seed<'a>(adapter: Option<&'a str>, dest: &Path) -> Result<&'a str, Error> {
    adapter.ok_or_else(|| Error::Diag {
        code: "workspace-greenfield-no-adapter-seed",
        detail: format!(
            "`{}` needs a greenfield scaffold but registry.yaml declares no `adapter` seed for \
             this project; add `adapter: <name@vN>` to the registry entry as a greenfield \
             seed, or create the repo with its own `.specify/project.yaml` first",
            dest.display()
        ),
    })
}

fn ensure_origin_matches(dest: &Path, expected_url: &str) -> Result<(), Error> {
    match git_output_ok(dest, &["remote", "get-url", "origin"]) {
        Some(actual) if actual == expected_url => Ok(()),
        Some(actual) => Err(Error::Diag {
            code: "workspace-origin-mismatch",
            detail: format!(
                "`{}` origin remote is `{actual}`, but registry url is `{expected_url}`; \
                 remove the slot or update registry.yaml before re-syncing",
                dest.display()
            ),
        }),
        None => Err(Error::Diag {
            code: "workspace-origin-missing",
            detail: format!(
                "`{}` has no origin remote; expected registry url `{expected_url}`",
                dest.display()
            ),
        }),
    }
}

/// Copy root `contracts/` from the initiating repo into a workspace slot's
/// root `contracts/`. Removes the destination first for a clean replacement,
/// then copies recursively.
pub(super) fn distribute_contracts(src: &Path, dest: &Path) -> Result<(), Error> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| Error::Diag {
            code: "workspace-contracts-remove-failed",
            detail: format!("failed to remove old contracts at {}: {e}", dest.display()),
        })?;
    }
    copy_dir_recursive(src, dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dest).map_err(|e| Error::Diag {
        code: "workspace-contracts-create-failed",
        detail: format!("failed to create {}: {e}", dest.display()),
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| Error::Diag {
        code: "workspace-contracts-read-failed",
        detail: format!("failed to read {}: {e}", src.display()),
    })? {
        let entry = entry.map_err(|e| Error::Diag {
            code: "workspace-contracts-entry-failed",
            detail: format!("dir entry error: {e}"),
        })?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|e| Error::Diag {
                code: "workspace-contracts-copy-failed",
                detail: format!(
                    "failed to copy {} to {}: {e}",
                    src_path.display(),
                    dest_path.display()
                ),
            })?;
        }
    }
    Ok(())
}
