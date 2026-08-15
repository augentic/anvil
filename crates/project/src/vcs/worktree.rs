//! The native D11 materialize leg behind the `emery:vcs/worktree`
//! seam: provision the publication checkout, apply the closed state
//! table, materialize the accepted CID, and stage the index.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::seam::{WorktreeError, WorktreeRequest, WorktreeState};
use crate::workspace::{FsObjects, Store};

/// The deployment surfaces one export runs over.
///
/// The snapshot store the CID materializes from, the
/// `$EMERY_HOME/publication/` slot root, and the in-place candidate
/// (the product checkout) when the deployment has one.
#[derive(Debug)]
pub struct ExportEnv<'a> {
    /// Snapshot store holding the accepted CID.
    pub store: &'a Store<FsObjects>,
    /// Slot root — worktrees live at `<root>/<plan>/<target>/`.
    pub publication_root: &'a Path,
    /// In-place candidate, honored only when the request carries
    /// `allow_in_place`. `None` for detached deployments.
    pub product_root: Option<&'a Path>,
}

/// One RFC-95 D11 materialize. Returns the worktree path plus the
/// idempotency state; refusals are the closed [`WorktreeError`] rows.
///
/// # Errors
///
/// The D11 state-table refusal rows.
pub async fn export(
    env: &ExportEnv<'_>, req: &WorktreeRequest,
) -> Result<(PathBuf, WorktreeState), WorktreeError> {
    validate(req)?;
    let dest = destination(env, req)?;
    ensure_exclude(&dest)?;
    ensure_parent(&dest, &req.parent_revision)?;
    match branch_state(&dest, req)? {
        Branch::OperatorCommitted => Ok((dest, WorktreeState::Matched)),
        Branch::Ready => {
            if operator_dirty(&dest)? {
                return Err(WorktreeError::Dirty);
            }
            let state = materialize(env, req, &dest).await?;
            Ok((dest, state))
        }
    }
}

/// Resolved branch situation after the provisioning rows.
enum Branch {
    /// `HEAD` moved off the recorded parent — the operator committed;
    /// leave everything (no-rewind).
    OperatorCommitted,
    /// The publication branch is checked out at the recorded parent.
    Ready,
}

fn validate(req: &WorktreeRequest) -> Result<(), WorktreeError> {
    let segment_ok = |s: &str| !s.is_empty() && !s.contains(['/', '\\']) && s != "." && s != "..";
    if !segment_ok(&req.plan) || !segment_ok(&req.target) {
        return Err(WorktreeError::InvalidRequest(format!(
            "plan `{}` / target `{}` are not slot path segments",
            req.plan, req.target
        )));
    }
    if req.branch.is_empty() || req.parent_revision.is_empty() || req.repository.is_empty() {
        return Err(WorktreeError::InvalidRequest(
            "repository, parent revision, and branch are required".into(),
        ));
    }
    Ok(())
}

/// D11 placement: the in-place candidate when eligible, else the
/// `$EMERY_HOME/publication/<plan>/<target>/` slot (cloning first
/// time, refusing a non-worktree squatter).
fn destination(env: &ExportEnv<'_>, req: &WorktreeRequest) -> Result<PathBuf, WorktreeError> {
    if req.allow_in_place
        && let Some(root) = env.product_root
        && in_place_eligible(root, req)?
    {
        return Ok(root.to_path_buf());
    }
    let slot = env.publication_root.join(&req.plan).join(&req.target);
    if !slot.exists() {
        clone(&req.repository, &slot)?;
        return Ok(slot);
    }
    if !slot.join(".git").exists() {
        return Err(WorktreeError::DestinationConflict);
    }
    Ok(slot)
}

/// The in-place rule: the product checkout is a Git repository that
/// is either already on the publication branch (re-entry) or clean at
/// the recorded parent (first time). Anything else falls to the slot.
fn in_place_eligible(root: &Path, req: &WorktreeRequest) -> Result<bool, WorktreeError> {
    if !root.join(".git").exists() {
        return Ok(false);
    }
    ensure_exclude(root)?;
    if current_branch(root)?.as_deref() == Some(req.branch.as_str()) {
        return Ok(true);
    }
    Ok(head(root).as_deref() == Some(req.parent_revision.as_str()) && pristine(root)?)
}

/// Resolve the publication branch per the closed state table, ending
/// with it checked out (or the operator's commit left alone).
fn branch_state(dest: &Path, req: &WorktreeRequest) -> Result<Branch, WorktreeError> {
    if current_branch(dest)?.as_deref() == Some(req.branch.as_str()) {
        if head(dest).as_deref() != Some(req.parent_revision.as_str()) {
            return Ok(Branch::OperatorCommitted);
        }
        return Ok(Branch::Ready);
    }
    let Some(commit) = branch_commit(dest, &req.branch) else {
        // The branch does not exist yet — create it at the parent.
        if !pristine(dest)? {
            return Err(WorktreeError::Dirty);
        }
        git(dest, &["checkout", "--quiet", "-b", &req.branch, &req.parent_revision])
            .map_err(WorktreeError::Internal)?;
        return Ok(Branch::Ready);
    };
    if checked_out_elsewhere(dest, &req.branch)? {
        return Err(WorktreeError::BranchCheckedOutElsewhere);
    }
    if commit != req.parent_revision {
        return Err(WorktreeError::BranchDiverged);
    }
    if !pristine(dest)? {
        return Err(WorktreeError::Dirty);
    }
    git(dest, &["checkout", "--quiet", &req.branch]).map_err(WorktreeError::Internal)?;
    Ok(Branch::Ready)
}

/// Replace the staged content with the accepted CID and stage the
/// index; the returned state compares the staged tree before/after.
async fn materialize(
    env: &ExportEnv<'_>, req: &WorktreeRequest, dest: &Path,
) -> Result<WorktreeState, WorktreeError> {
    let parent_tree = git(dest, &["rev-parse", &format!("{}^{{tree}}", req.parent_revision)])
        .map_err(WorktreeError::Internal)?;
    let before = write_tree(dest)?;
    remove_tracked(dest)?;
    env.store
        .materialize(&req.cid, dest)
        .await
        .map_err(|err| WorktreeError::Internal(format!("materializing the accepted CID: {err}")))?;
    git(dest, &["add", "-A"]).map_err(WorktreeError::Internal)?;
    let after = write_tree(dest)?;
    // Record what the engine staged so a later pass can tell its own
    // staging apart from operator-staged edits (the dirty check).
    git(dest, &["update-ref", PUBLICATION_REF, &after]).map_err(WorktreeError::Internal)?;
    if after == before && before != parent_tree {
        return Ok(WorktreeState::Matched);
    }
    if before == parent_tree {
        return Ok(WorktreeState::Created);
    }
    Ok(WorktreeState::Rematerialized)
}

/// Remove every tracked path except the nested change home, then
/// prune the directories the removals emptied.
fn remove_tracked(dest: &Path) -> Result<(), WorktreeError> {
    let listing = git(dest, &["ls-files", "-z"]).map_err(WorktreeError::Internal)?;
    for rel in listing.split('\0').filter(|rel| !rel.is_empty()) {
        if rel.starts_with(".emery/change/") {
            continue;
        }
        let path = dest.join(rel);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(WorktreeError::Internal(format!("removing {}: {err}", path.display())));
            }
        }
    }
    prune_empty_dirs(dest, dest);
    Ok(())
}

/// Best-effort bottom-up removal of empty directories beneath `dir`,
/// never entering `.git` and never removing the root.
fn prune_empty_dirs(root: &Path, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && !path.is_symlink() && entry.file_name() != ".git" {
            prune_empty_dirs(root, &path);
        }
    }
    if dir != root {
        drop(std::fs::remove_dir(dir));
    }
}

/// The node-local ref recording the tree the engine last staged —
/// the yardstick telling engine staging apart from operator edits.
const PUBLICATION_REF: &str = "refs/emery/publication";

/// Whether the operator has uncommitted edits: unstaged or untracked
/// state, or a staged tree that is not the engine's own recorded
/// materialization (an operator-staged edit is dirty too).
fn operator_dirty(dest: &Path) -> Result<bool, WorktreeError> {
    let rows = status(dest)?;
    if rows.iter().any(|row| {
        let mut chars = row.chars();
        let staged = chars.next().unwrap_or(' ');
        let unstaged = chars.next().unwrap_or(' ');
        staged == '?' || unstaged != ' '
    }) {
        return Ok(true);
    }
    if rows.is_empty() {
        return Ok(false);
    }
    let recorded = git(dest, &["rev-parse", "--verify", "--quiet", PUBLICATION_REF]).ok();
    Ok(recorded.as_deref() != Some(write_tree(dest)?.as_str()))
}

/// Whether the checkout carries no staged, unstaged, or untracked
/// state at all — the bar for switching branches and for first-time
/// in-place placement.
fn pristine(dest: &Path) -> Result<bool, WorktreeError> {
    Ok(status(dest)?.is_empty())
}

fn status(dest: &Path) -> Result<Vec<String>, WorktreeError> {
    let listing = git(dest, &["status", "--porcelain"]).map_err(WorktreeError::Internal)?;
    Ok(listing.lines().map(ToString::to_string).collect())
}

/// Confirm the recorded parent resolves to a commit, fetching once
/// when it does not.
fn ensure_parent(dest: &Path, parent: &str) -> Result<(), WorktreeError> {
    let probe = format!("{parent}^{{commit}}");
    if git(dest, &["rev-parse", "--verify", "--quiet", &probe]).is_ok() {
        return Ok(());
    }
    drop(git(dest, &["fetch", "--quiet", "--all"]));
    if git(dest, &["rev-parse", "--verify", "--quiet", &probe]).is_ok() {
        return Ok(());
    }
    Err(WorktreeError::ParentUnavailable)
}

/// Keep the nested change home out of Git status and staging without
/// touching tracked `.gitignore` content.
fn ensure_exclude(dest: &Path) -> Result<(), WorktreeError> {
    const ENTRY: &str = "/.emery/change/";
    let exclude = dest.join(".git/info/exclude");
    let current = std::fs::read_to_string(&exclude).unwrap_or_default();
    if current.lines().any(|line| line.trim() == ENTRY) {
        return Ok(());
    }
    if let Some(parent) = exclude.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            WorktreeError::Internal(format!("creating {}: {err}", parent.display()))
        })?;
    }
    std::fs::write(&exclude, format!("{current}{ENTRY}\n"))
        .map_err(|err| WorktreeError::Internal(format!("writing {}: {err}", exclude.display())))
}

fn clone(repository: &str, dest: &Path) -> Result<(), WorktreeError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            WorktreeError::Internal(format!("creating {}: {err}", parent.display()))
        })?;
    }
    let output = Command::new("git")
        .args(["clone", "--quiet", repository])
        .arg(dest)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|err| WorktreeError::Internal(format!("spawning git: {err}")))?;
    if !output.status.success() {
        return Err(WorktreeError::CloneFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// The checked-out branch name, or `None` when `HEAD` is detached or
/// unborn.
fn current_branch(dest: &Path) -> Result<Option<String>, WorktreeError> {
    let name =
        git(dest, &["rev-parse", "--abbrev-ref", "HEAD"]).map_err(WorktreeError::Internal)?;
    Ok((name != "HEAD" && !name.is_empty()).then_some(name))
}

fn head(dest: &Path) -> Option<String> {
    git(dest, &["rev-parse", "--verify", "--quiet", "HEAD"]).ok()
}

/// The commit a local branch points at, or `None` when the branch
/// does not exist.
fn branch_commit(dest: &Path, branch: &str) -> Option<String> {
    git(dest, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).ok()
}

/// Whether `branch` is checked out in another linked worktree of the
/// same repository.
fn checked_out_elsewhere(dest: &Path, branch: &str) -> Result<bool, WorktreeError> {
    let listing =
        git(dest, &["worktree", "list", "--porcelain"]).map_err(WorktreeError::Internal)?;
    let needle = format!("branch refs/heads/{branch}");
    let dest_line = format!("worktree {}", dest.display());
    let mut in_this_worktree = false;
    for line in listing.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            in_this_worktree = format!("worktree {rest}") == dest_line;
        } else if line == needle && !in_this_worktree {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The staged tree's object id — the before/after comparison behind
/// the idempotency state.
fn write_tree(dest: &Path) -> Result<String, WorktreeError> {
    git(dest, &["write-tree"]).map_err(WorktreeError::Internal)
}

/// Run one prompt-free `git` invocation in `dest`, returning trimmed
/// stdout.
fn git(dest: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dest)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|err| format!("spawning git: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
