//! Thin shells around `git` for workspace materialisation, status, and push.

use std::path::Path;

use specify_error::Error;

use crate::cmd;

pub(super) fn git_output_ok(tree: &Path, args: &[&str]) -> Option<String> {
    let output = cmd::git(&cmd::real_cmd, Some(tree), args).ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub(super) fn git_porcelain_non_empty(tree: &Path) -> bool {
    let Ok(output) = cmd::git(&cmd::real_cmd, Some(tree), ["status", "--porcelain"]) else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    !output.stdout.is_empty()
}

pub(super) fn run(cwd: &Path, args: &[&str], label: &str) -> Result<(), Error> {
    cmd::git_as_specify_checked(&cmd::real_cmd, cwd, args)
        .map(|_output| ())
        .map_err(|failure| classify_workspace_failure(failure, label))
}

/// Map a classified git failure onto the `workspace-git-*` family.
fn classify_workspace_failure(failure: cmd::GitFailure, label: &str) -> Error {
    match failure {
        cmd::GitFailure::Spawn(err) => Error::Diag {
            code: "workspace-git-spawn-failed",
            detail: format!("{label}: failed to spawn git: {err}"),
        },
        cmd::GitFailure::Exit { stderr } => Error::Diag {
            code: "workspace-git-command-failed",
            detail: format!("{label} failed: {stderr}"),
        },
    }
}

pub(super) fn git_status_porcelain(project_path: &Path) -> Result<String, Error> {
    git_stdout_allow_empty(
        project_path,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "git status --porcelain",
    )
}

pub(super) fn git_stdout_trimmed(
    project_path: &Path, args: &[&str], label: &str,
) -> Result<String, Error> {
    let stdout = git_stdout_allow_empty(project_path, args, label)?;
    let trimmed = stdout.trim().to_string();
    if trimmed.is_empty() {
        return Err(Error::Diag {
            code: "workspace-git-empty-output",
            detail: format!("{label} returned no output"),
        });
    }
    Ok(trimmed)
}

pub(super) fn git_stdout_allow_empty(
    project_path: &Path, args: &[&str], label: &str,
) -> Result<String, Error> {
    cmd::git_checked(&cmd::real_cmd, Some(project_path), args)
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .map_err(|failure| classify_workspace_failure(failure, label))
}
