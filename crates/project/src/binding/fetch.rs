//! Bind one locator through the seams (RFC-88 over RFC-95): skips
//! engine-side, origin I/O through [`Trees`], CIDs via
//! [`Workspaces`]. Wasm-clean — the host only stages trees.

use std::path::PathBuf;

use error::Error;

use super::ingest::{Resolved, Session, Staged};
use super::locator::{Location, Locator};
use crate::seam::{self, TreeCredentials, TreeError, TreeLimits, Trees, Workspaces};
use crate::snapshot::SnapshotId;

/// Bind one parsed locator: skips, seam fetch, then the wasm-clean
/// CID kernel. Returns the resolved pin and the staged tree path for
/// fingerprinting.
///
/// # Errors
///
/// Locator, fetch, budget, and snapshot failures.
pub async fn resolve<W: Workspaces, T: Trees>(
    session: &mut Session<'_, W>, trees: &T, location: &Location, recorded: Option<&SnapshotId>,
    prior: Option<&str>,
) -> Result<(Resolved, PathBuf), Error> {
    match &location.locator {
        Locator::Path(_) => {
            let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
            let root = path_root(session.change_root, location)?;
            Ok((resolved, root))
        }
        Locator::Git { url, revision } => {
            if let Some(cid) = recorded
                && session.workspaces.contains(cid.clone()).await.unwrap_or(false)
            {
                let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
                return Ok((
                    resolved,
                    path_root(session.change_root, location)
                        .unwrap_or_else(|_| session.scratch.to_path_buf()),
                ));
            }
            if let Some(cid) = session.cache.get(location)
                && session.workspaces.contains(cid.clone()).await.unwrap_or(false)
            {
                let resolved = session.ingest(location, Staged::Disk, Some(&cid), None).await?;
                return Ok((resolved, session.scratch.to_path_buf()));
            }
            let (root, exact, warning) = stage_git(session, trees, url, revision, prior).await?;
            let mut pinned = location.clone();
            pinned.locator = Locator::Git {
                url: url.clone(),
                revision: exact,
            };
            let resolved = session.ingest(&pinned, Staged::Tree(&root), recorded, warning).await?;
            session.cache.insert(location, resolved.cid.clone());
            Ok((resolved, root))
        }
        Locator::Https(url) => {
            if let Some(cid) = recorded
                && session.workspaces.contains(cid.clone()).await.unwrap_or(false)
            {
                let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
                return Ok((resolved, session.scratch.to_path_buf()));
            }
            if let Some(cid) = session.cache.get(location)
                && session.workspaces.contains(cid.clone()).await.unwrap_or(false)
            {
                let resolved = session.ingest(location, Staged::Disk, Some(&cid), None).await?;
                return Ok((resolved, session.scratch.to_path_buf()));
            }
            session.meter.api(session.policy)?;
            let staged = trees
                .fetch(url.clone(), TreeCredentials::None, TreeLimits::standard(session.policy))
                .await
                .map_err(tree_failure)?;
            let root = PathBuf::from(&staged.root);
            let resolved = session.ingest(location, Staged::Tree(&root), recorded, None).await?;
            Ok((resolved, root))
        }
    }
}

/// Stage a Git locator through the seam and settle the exact commit.
///
/// A mutable ref resolves to its tip host-side; when a recorded
/// `prior` SHA exists and the tip moved, the engine re-pins to the
/// recorded commit (a second seam fetch) and carries the freshness
/// warning — the RFC-88 moved-branch comparison is an engine check.
async fn stage_git<W: Workspaces, T: Trees>(
    session: &mut Session<'_, W>, trees: &T, url: &str, revision: &str, prior: Option<&str>,
) -> Result<(PathBuf, String, Option<String>), Error> {
    let limits = TreeLimits::standard(session.policy);
    session.meter.api(session.policy)?;
    let staged = trees
        .fetch(git_locator(url, revision), TreeCredentials::None, limits)
        .await
        .map_err(tree_failure)?;
    let exact = staged.revision.clone().ok_or_else(|| Error::Diag {
        code: "binding-fetch-failed",
        detail: format!("the fetch of `{url}@{revision}` reported no commit"),
    })?;
    if let Some(prior) = prior
        && !Locator::is_sha(revision)
        && exact != prior
    {
        let warning = format!(
            "git ref `{revision}` moved from `{prior}` to `{exact}`; ingesting recorded commit"
        );
        drop(trees.discard_fetched(staged.root).await);
        session.meter.api(session.policy)?;
        let pinned = trees
            .fetch(git_locator(url, prior), TreeCredentials::None, limits)
            .await
            .map_err(tree_failure)?;
        return Ok((PathBuf::from(&pinned.root), prior.to_string(), Some(warning)));
    }
    Ok((PathBuf::from(&staged.root), exact, None))
}

fn git_locator(url: &str, revision: &str) -> String {
    Locator::Git {
        url: url.to_string(),
        revision: revision.to_string(),
    }
    .key()
}

fn path_root(change_root: &std::path::Path, location: &Location) -> Result<PathBuf, Error> {
    let Locator::Path(path) = &location.locator else {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: "disk ingest requires a path locator".into(),
        });
    };
    let base = if path.is_absolute() { path.clone() } else { change_root.join(path) };
    if location.path == "." {
        return Ok(base);
    }
    Ok(base.join(&location.path))
}

/// Map a seam tree-fetch failure onto the bind's diagnostic contract.
fn tree_failure(err: TreeError) -> Error {
    match err {
        TreeError::InvalidRequest(detail) => Error::Diag {
            code: "locator-malformed",
            detail,
        },
        TreeError::Access(detail) | TreeError::Internal(detail) => Error::Diag {
            code: "binding-fetch-failed",
            detail,
        },
        TreeError::Limit(detail) => Error::Diag {
            code: "binding-budget-exhausted",
            detail,
        },
    }
}

/// Fetch a locator string into a seam [`crate::seam::Fetched`].
///
/// # Errors
///
/// Locator parse, fetch, budget, and snapshot failures.
pub async fn fetch<W: Workspaces, T: Trees>(
    session: &mut Session<'_, W>, trees: &T, locator: &str, recorded: Option<&SnapshotId>,
    prior: Option<&str>,
) -> Result<seam::Fetched, Error> {
    let location = Location::parse(locator, None)?;
    let (resolved, root) = resolve(session, trees, &location, recorded, prior).await?;
    Ok(seam::Fetched {
        locator: resolved.location.locator.key(),
        cid: resolved.cid,
        root: root.display().to_string(),
        warning: resolved.warning,
    })
}
