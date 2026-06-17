//! Orchestrator for one slot's branch preparation: derive the target
//! branch, validate the worktree, then create-or-reuse the local
//! branch and fast-forward against the remote where possible.

use std::path::Path;

use super::infer::{
    project_worktree_path, refresh_origin_head, resolve_origin_head, target_branch,
};
use super::validate::{classify_dirty, require_git_worktree, require_origin};
use super::{
    Diagnostic, LocalAction, Prepared, RemoteAction, Request, git_output, git_output_optional,
    git_success, run_git,
};
use crate::registry::catalog::RegistryProject;

/// Prepare a resolved registry project's worktree on `specify/<change-name>`.
///
/// # Errors
///
/// Returns a structured diagnostic when the slot is missing, the remote default
/// cannot be resolved, the branch name is outside the expected pattern, unrelated
/// tracked work is dirty, or a required Git operation fails.
pub fn prepare(
    project_dir: &Path, project: &RegistryProject, request: &Request,
) -> Result<Prepared, Diagnostic> {
    let branch = target_branch(project, &request.change_name)?;
    let slot_path = project_worktree_path(project_dir, project);
    require_git_worktree(&slot_path, project, &branch)?;
    let remote_url = require_origin(&slot_path, project, &branch)?;
    if !project.is_local() && remote_url != project.url {
        return Err(Diagnostic::new(
            "origin-mismatch",
            project,
            Some(&branch),
            format!(
                "`{}` origin remote is `{remote_url}`, but registry url is `{}`",
                slot_path.display(),
                project.url
            ),
        ));
    }

    run_git(&slot_path, ["fetch", "origin"], project, Some(&branch), "git fetch origin")?;
    refresh_origin_head(&slot_path);
    let base_ref = resolve_origin_head(&slot_path, project, &branch)?;
    let base_sha = git_output(&slot_path, ["rev-parse", "origin/HEAD"], project, Some(&branch))?;

    let current_branch =
        git_output_optional(&slot_path, ["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let local_exists = git_success(
        &slot_path,
        ["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")],
    );
    let dirty = classify_dirty(
        &slot_path,
        &request.change_name,
        &request.source_paths,
        &request.output_paths,
    );

    if !dirty.tracked_blocked.is_empty() {
        return Err(Diagnostic::new(
            "dirty-unrelated-tracked",
            project,
            Some(&branch),
            "tracked work outside the active slice boundary blocks branch preparation",
        )
        .with_paths(dirty.tracked_blocked));
    }

    if dirty.has_allowed_tracked() && current_branch.as_deref() != Some(branch.as_str()) {
        return Err(Diagnostic::new(
            "dirty-branch-mismatch",
            project,
            Some(&branch),
            "resume-safe tracked work is allowed only when already on the change branch",
        )
        .with_paths(dirty.tracked_allowed));
    }

    let local_branch = if local_exists {
        if current_branch.as_deref() != Some(branch.as_str()) {
            run_git(
                &slot_path,
                ["checkout", &branch],
                project,
                Some(&branch),
                &format!("git checkout {branch}"),
            )?;
        }
        LocalAction::Reused
    } else {
        run_git(
            &slot_path,
            ["checkout", "-b", &branch, "origin/HEAD"],
            project,
            Some(&branch),
            &format!("git checkout -b {branch} origin/HEAD"),
        )?;
        LocalAction::Created
    };

    let remote_branch = fast_forward_remote_branch(&slot_path, project, &branch)?;

    Ok(Prepared {
        project: project.name.clone(),
        slot_path: slot_path.to_string_lossy().into_owned(),
        branch,
        base_ref,
        base_sha,
        local_branch,
        remote_branch,
        dirty,
    })
}

fn fast_forward_remote_branch(
    slot_path: &Path, project: &RegistryProject, branch: &str,
) -> Result<RemoteAction, Diagnostic> {
    let remote_ref = format!("refs/remotes/origin/{branch}");
    if !git_success(slot_path, ["show-ref", "--verify", "--quiet", &remote_ref]) {
        return Ok(RemoteAction::Absent);
    }

    let local = git_output(slot_path, ["rev-parse", "HEAD"], project, Some(branch))?;
    let remote =
        git_output(slot_path, ["rev-parse", &format!("origin/{branch}")], project, Some(branch))?;
    if local == remote {
        return Ok(RemoteAction::UpToDate);
    }
    if git_success(slot_path, ["merge-base", "--is-ancestor", "HEAD", &format!("origin/{branch}")])
    {
        run_git(
            slot_path,
            ["merge", "--ff-only", &format!("origin/{branch}")],
            project,
            Some(branch),
            &format!("git merge --ff-only origin/{branch}"),
        )?;
        return Ok(RemoteAction::FastForwarded);
    }
    if git_success(slot_path, ["merge-base", "--is-ancestor", &format!("origin/{branch}"), "HEAD"])
    {
        return Ok(RemoteAction::LocalAhead);
    }
    Err(Diagnostic::new(
        "remote-branch-diverged",
        project,
        Some(branch),
        format!("local `{branch}` and `origin/{branch}` have diverged; reconcile manually"),
    ))
}
