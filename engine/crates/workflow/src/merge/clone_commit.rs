//! Workspace-clone auto-commit for `slice merge run`.
//!
//! When a merge runs inside a materialised workspace slot, the
//! baseline spec tree and archived slice are committed back to the
//! slot's git clone so the workspace stays consistent. The git side
//! effects live here (out of the binary handler per
//! [`handler-shape.md`](../../../../docs/standards/handler-shape.md));
//! warnings are returned as data for the handler to render, never
//! printed from inside the engine.

use std::path::Path;
use std::process::Output;

use crate::config::{Layout, is_slot};

/// Baseline paths the merge-owned workspace commit is limited to.
/// Opaque/generated outputs remain as residue for the execute driver.
const WORKSPACE_MERGE_COMMIT_PATHS: [&str; 2] = [".specify/specs", ".specify/archive"];

/// Detect whether a project directory is inside a workspace clone.
///
/// The path must sit at or below a `workspace/<peer>/` slot (see
/// [`is_slot`]), and `.specify/plan.yaml` must be absent — the plan
/// file's presence indicates an in-flight change rather than a freshly
/// merged clone. The `.specify/project.yaml` check is already enforced
/// upstream by `Ctx::load`.
#[must_use]
pub fn is_clone_eligible(project_dir: &Path) -> bool {
    if !is_slot(project_dir) {
        return false;
    }
    !Layout::new(project_dir).plan_path().exists()
}

/// Read the current git HEAD SHA, or `None` when the project is not a
/// git repository or `git` is unavailable.
#[must_use]
pub fn head_sha(project_dir: &Path) -> Option<String> {
    let output = git(project_dir, &["rev-parse", "HEAD"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

/// Commit the baseline spec tree and archived slice back to the
/// workspace slot's git clone after a merge.
///
/// Returns the warnings (already `warning: …`-prefixed) the handler
/// should render to stderr. An empty `Vec` means a clean run — nothing
/// to commit, or the commit succeeded. Best-effort throughout: every
/// git hiccup becomes a returned warning rather than a failure, so a
/// committed merge is never undone by a workspace-commit problem.
#[must_use]
pub fn auto_commit(project_dir: &Path, name: &str) -> Vec<String> {
    let pathspecs: Vec<&'static str> = WORKSPACE_MERGE_COMMIT_PATHS
        .iter()
        .copied()
        .filter(|path| project_dir.join(path).exists())
        .collect();
    if pathspecs.is_empty() {
        return Vec::new();
    }
    let mut add_args = vec!["add", "--"];
    add_args.extend(pathspecs.iter().copied());
    let add = match git(project_dir, &add_args) {
        Ok(output) => output,
        Err(err) => return vec![format!("warning: workspace auto-commit git-add: {err}")],
    };
    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        return vec![format!("warning: workspace auto-commit git-add: {stderr}")];
    }

    let mut diff_args = vec!["diff", "--cached", "--quiet", "--"];
    diff_args.extend(pathspecs.iter().copied());
    match git(project_dir, &diff_args).map(|o| o.status) {
        Ok(status) if status.success() => return Vec::new(),
        Ok(status) if status.code() == Some(1) => {}
        Ok(status) => {
            return vec![format!("warning: workspace auto-commit diff check: status {status}")];
        }
        Err(err) => return vec![format!("warning: workspace auto-commit diff check: {err}")],
    }

    let commit_msg = format!("specify: merge {name}");
    let mut commit_args = vec!["commit", "-m", &commit_msg, "--"];
    commit_args.extend(pathspecs.iter().copied());
    match git(project_dir, &commit_args) {
        Ok(commit) if !commit.status.success() => {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            vec![format!("warning: workspace auto-commit commit: {stderr}")]
        }
        Ok(_) => Vec::new(),
        Err(err) => vec![format!("warning: workspace auto-commit commit: {err}")],
    }
}

fn git(project_dir: &Path, args: &[&str]) -> std::io::Result<Output> {
    crate::cmd::git(&crate::cmd::real_cmd, Some(project_dir), args)
}

#[cfg(test)]
mod tests;
