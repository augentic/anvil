//! `specify workspace push`: publish the prepared `specify/<change>`
//! branch to each project's `origin`. Pull-request creation is
//! operator-owned and intentionally out of scope — push lands the
//! branch and stops.

pub(in crate::registry::workspace) mod remote;

use std::path::Path;

use specify_error::Error;

use self::remote::{current_branch, is_git_worktree, remote_branch_head, remote_default_branch_is};
use super::git::{self, git_output_ok, git_status_porcelain, git_stdout_trimmed};
use super::workspace_base;
use crate::registry::catalog::RegistryProject;

/// Resolve the on-disk path for a workspace project.
///
/// Symlink projects use the original path (relative or `.`), everything
/// else lives under `workspace/<name>/`.
fn project_path(
    project_dir: &Path, workspace_base: &Path, rp: &RegistryProject,
) -> std::path::PathBuf {
    if rp.is_local() {
        if rp.url == "." { project_dir.to_path_buf() } else { project_dir.join(&rp.url) }
    } else {
        workspace_base.join(&rp.name)
    }
}

/// Classification of a single project push outcome.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PushOutcome {
    /// Branch pushed to remote.
    Pushed,
    /// Push failed (see `PushResult.error`).
    Failed,
    /// No changes to push.
    UpToDate,
    /// Local-only project (no remote configured).
    LocalOnly,
    /// Checkout is not currently on the expected `specify/<change>` branch.
    NoBranch,
}

/// Result of a per-project push operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PushResult {
    /// Registry project name.
    pub name: String,
    /// Outcome of this push.
    pub status: PushOutcome,
    /// Git branch pushed to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Human-readable error when the push failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Core implementation of `specify workspace push` for pre-resolved projects.
///
/// # Errors
///
/// Per-project push outcomes are returned in the result vector; the outer
/// `Result` is reserved for fatal errors that prevent any push from running.
pub fn push_projects(
    project_dir: &Path, change_name: &str, target_projects: &[&RegistryProject], dry_run: bool,
) -> Result<Vec<PushResult>, Error> {
    let branch_name = format!("specify/{change_name}");
    let workspace_base = workspace_base(project_dir);

    let mut results = Vec::new();

    for rp in target_projects {
        let result = push_single_project(project_dir, &workspace_base, rp, &branch_name, dry_run);
        results.push(result);
    }

    Ok(results)
}

fn push_branch(
    project_path: &Path, branch_name: &str, expected_remote_head: Option<&str>,
) -> Result<(), Error> {
    let lease = expected_remote_head.map_or_else(
        || format!("--force-with-lease=refs/heads/{branch_name}:"),
        |sha| format!("--force-with-lease=refs/heads/{branch_name}:{sha}"),
    );
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");
    git::run(
        project_path,
        &["push", &lease, "-u", "origin", &refspec],
        &format!("git push to {branch_name}"),
    )
}

fn push_result(
    rp: &RegistryProject, status: PushOutcome, branch: Option<&str>, error: Option<String>,
) -> PushResult {
    PushResult {
        name: rp.name.clone(),
        status,
        branch: branch.map(ToString::to_string),
        error,
    }
}

struct Ready<'a> {
    rp: &'a RegistryProject,
    project_path: std::path::PathBuf,
    branch_name: &'a str,
    local_head: String,
    remote_head: Option<String>,
}

pub(in crate::registry::workspace) fn push_single_project(
    project_dir: &Path, workspace_base: &Path, rp: &RegistryProject, branch_name: &str,
    dry_run: bool,
) -> PushResult {
    let ready = match prepare_push(project_dir, workspace_base, rp, branch_name) {
        Ok(r) => r,
        Err(result) => return result,
    };
    publish_push(ready, dry_run)
}

fn prepare_push<'a>(
    project_dir: &Path, workspace_base: &Path, rp: &'a RegistryProject, branch_name: &'a str,
) -> Result<Ready<'a>, PushResult> {
    let project_path = project_path(project_dir, workspace_base, rp);

    if !is_git_worktree(&project_path) {
        return Err(push_result(
            rp,
            PushOutcome::Failed,
            None,
            Some(format!("no git worktree found at {}", project_path.display())),
        ));
    }

    if git_output_ok(&project_path, &["remote", "get-url", "origin"]).is_none() {
        return Err(push_result(rp, PushOutcome::LocalOnly, None, None));
    }

    match current_branch(&project_path) {
        Ok(Some(current)) if current == branch_name => {}
        Ok(_) => return Err(push_result(rp, PushOutcome::NoBranch, None, None)),
        Err(err) => {
            return Err(push_result(rp, PushOutcome::Failed, None, Some(err.to_string())));
        }
    }

    match git_status_porcelain(&project_path) {
        Ok(status) if status.is_empty() => {}
        Ok(_) => {
            return Err(push_result(
                rp,
                PushOutcome::Failed,
                Some(branch_name),
                Some("checkout is dirty; commit or clean local work before workspace push".into()),
            ));
        }
        Err(err) => {
            return Err(push_result(
                rp,
                PushOutcome::Failed,
                Some(branch_name),
                Some(err.to_string()),
            ));
        }
    }

    if remote_default_branch_is(&project_path, branch_name) {
        return Err(push_result(rp, PushOutcome::NoBranch, None, None));
    }

    let local_head =
        match git_stdout_trimmed(&project_path, &["rev-parse", "HEAD"], "rev-parse HEAD") {
            Ok(sha) => sha,
            Err(err) => {
                return Err(push_result(
                    rp,
                    PushOutcome::Failed,
                    Some(branch_name),
                    Some(err.to_string()),
                ));
            }
        };

    let remote_head = match remote_branch_head(&project_path, branch_name) {
        Ok(head) => head,
        Err(err) => {
            return Err(push_result(
                rp,
                PushOutcome::Failed,
                Some(branch_name),
                Some(err.to_string()),
            ));
        }
    };

    Ok(Ready {
        rp,
        project_path,
        branch_name,
        local_head,
        remote_head,
    })
}

fn publish_push(ready: Ready<'_>, dry_run: bool) -> PushResult {
    let Ready {
        rp,
        project_path,
        branch_name,
        local_head,
        remote_head,
    } = ready;
    let remote_head_ref = remote_head.as_deref();

    if remote_head_ref == Some(local_head.as_str()) {
        return push_result(rp, PushOutcome::UpToDate, Some(branch_name), None);
    }

    if dry_run {
        return push_result(rp, PushOutcome::Pushed, Some(branch_name), None);
    }

    if let Err(e) = push_branch(&project_path, branch_name, remote_head_ref) {
        return push_result(rp, PushOutcome::Failed, Some(branch_name), Some(e.to_string()));
    }

    push_result(rp, PushOutcome::Pushed, Some(branch_name), None)
}
