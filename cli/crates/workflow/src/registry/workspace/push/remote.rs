//! Remote-branch inspection helpers for `specify workspace push`: tip
//! discovery and default-branch resolution. Push publishes the prepared
//! `specify/<change>` branch only — PR creation is operator-owned and
//! lives outside the CLI.

use std::path::Path;

use specify_error::Error;

use crate::cmd;
use crate::registry::workspace::git::git_output_ok;

pub(super) fn is_git_worktree(project_path: &Path) -> bool {
    git_output_ok(project_path, &["rev-parse", "--is-inside-work-tree"]).as_deref() == Some("true")
}

pub(in crate::registry::workspace) fn current_branch(
    project_path: &Path,
) -> Result<Option<String>, Error> {
    let output = cmd::git(
        &cmd::real_cmd,
        Some(project_path),
        ["symbolic-ref", "--quiet", "--short", "HEAD"],
    )
    .map_err(|err| Error::Diag {
        code: "workspace-git-current-branch-failed",
        detail: format!("failed to inspect current branch: {err}"),
    })?;
    if !output.status.success() {
        return Ok(None);
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!branch.is_empty()).then_some(branch))
}

/// Resolve the remote tip of `branch_name` on `origin`, returning `None`
/// when the branch is absent.
///
/// # Errors
///
/// Returns `Error::Diag` when `git ls-remote` cannot run or exits
/// non-zero (e.g. the remote repository does not exist); push surfaces
/// that as a per-project `Failed` outcome.
pub(super) fn remote_branch_head(
    project_path: &Path, branch_name: &str,
) -> Result<Option<String>, Error> {
    let output = cmd::git(
        &cmd::real_cmd,
        Some(project_path),
        ["ls-remote", "--heads", "origin", &format!("refs/heads/{branch_name}")],
    )
    .map_err(|err| Error::Diag {
        code: "workspace-git-ls-remote-spawn-failed",
        detail: format!("failed to inspect remote branch: {err}"),
    })?;
    if !output.status.success() {
        return Err(Error::Diag {
            code: "workspace-git-ls-remote-failed",
            detail: format!(
                "git ls-remote origin {branch_name} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().find_map(|line| line.split_whitespace().next()).map(ToString::to_string))
}

pub(super) fn remote_default_branch_is(project_path: &Path, branch_name: &str) -> bool {
    if origin_head_branch(project_path).as_deref() == Some(branch_name) {
        return true;
    }

    drop(cmd::git(&cmd::real_cmd, Some(project_path), ["remote", "set-head", "origin", "--auto"]));

    origin_head_branch(project_path).as_deref() == Some(branch_name)
}

pub(in crate::registry::workspace) fn origin_head_branch(project_path: &Path) -> Option<String> {
    if let Some(full) =
        git_output_ok(project_path, &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"])
    {
        return full
            .strip_prefix("refs/remotes/origin/")
            .or_else(|| full.strip_prefix("origin/"))
            .map(ToString::to_string);
    }

    let output =
        cmd::git(&cmd::real_cmd, Some(project_path), ["ls-remote", "--symref", "origin", "HEAD"])
            .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        let rest = line.strip_prefix("ref: ")?;
        let (reference, target) = rest.split_once(char::is_whitespace)?;
        if target.trim() != "HEAD" {
            return None;
        }
        reference
            .strip_prefix("refs/heads/")
            .or_else(|| reference.strip_prefix("refs/remotes/origin/"))
            .or_else(|| reference.strip_prefix("origin/"))
            .map(ToString::to_string)
    })
}
