//! Deterministic workspace branch preparation. [`prepare()`] resolves
//! the workspace slot for one registry project, checks out or creates
//! `specify/<change-name>`, and reports the action taken.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::registry::catalog::RegistryProject;

mod infer;
mod prepare;
mod validate;

pub use prepare::prepare;

/// Inputs that define the branch-preparation dirtiness boundary.
#[derive(Debug, Clone)]
pub struct Request {
    /// Kebab-case umbrella change name. The target branch is exactly
    /// `specify/<change_name>`.
    pub change_name: String,
    /// Active plan-entry source paths that belong to this slice.
    pub source_paths: Vec<PathBuf>,
    /// Adapter-owned output paths that belong to this slice.
    pub output_paths: Vec<PathBuf>,
}

/// Successful branch-preparation result for one workspace slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Prepared {
    /// Registry project name.
    pub project: String,
    /// Prepared worktree path.
    pub slot_path: String,
    /// Target branch, always `specify/<change-name>`.
    pub branch: String,
    /// Remote-default symbolic ref used as the branch base.
    pub base_ref: String,
    /// Commit SHA of `origin/HEAD` after fetch/default-head resolution.
    pub base_sha: String,
    /// Whether the local branch was created or reused.
    pub local_branch: LocalAction,
    /// What happened with `origin/specify/<change-name>`.
    pub remote_branch: RemoteAction,
    /// Tracked/untracked dirtiness classification observed before checkout.
    pub dirty: Dirty,
}

/// Local branch action taken during preparation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LocalAction {
    /// A new local branch was created from `origin/HEAD`.
    Created,
    /// An existing local branch was checked out or already current.
    Reused,
}

/// Remote change-branch action taken during preparation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum RemoteAction {
    /// No `origin/specify/<change-name>` exists.
    Absent,
    /// The local branch already matched the remote branch.
    UpToDate,
    /// The local branch fast-forwarded to the remote branch.
    FastForwarded,
    /// The local branch is ahead of the remote branch.
    LocalAhead,
}

/// Dirty-state classification used by branch preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Dirty {
    /// Tracked dirty paths that are safe for slice resume.
    pub tracked_allowed: Vec<String>,
    /// Tracked dirty paths outside the active slice boundary.
    pub tracked_blocked: Vec<String>,
    /// Untracked paths. These do not block branch preparation but remain
    /// visible so push/status can refuse dirty checkouts later.
    pub untracked: Vec<String>,
    /// Relative path prefixes used for the allowed tracked classification.
    pub allowed_paths: Vec<String>,
}

impl Dirty {
    #[must_use]
    pub(super) const fn has_allowed_tracked(&self) -> bool {
        !self.tracked_allowed.is_empty()
    }
}

/// Machine-readable failure emitted before any unsafe branch mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diagnostic {
    /// Stable diagnostic key.
    pub key: String,
    /// Registry project name.
    pub project: String,
    /// Human-readable diagnostic.
    pub message: String,
    /// Target branch when it could be derived safely.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Relevant paths for dirty-path diagnostics.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

impl Diagnostic {
    #[must_use]
    pub(super) fn new(
        key: impl Into<String>, project: &RegistryProject, branch: Option<&str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            project: project.name.clone(),
            message: message.into(),
            branch: branch.map(ToString::to_string),
            paths: Vec::new(),
        }
    }

    #[must_use]
    pub(super) fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }
}

fn run_git<I, S>(
    cwd: &Path, args: I, project: &RegistryProject, branch: Option<&str>, label: &str,
) -> Result<(), Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    crate::cmd::git_checked(&crate::cmd::real_cmd, Some(cwd), args).map(|_output| ()).map_err(
        |failure| {
            let detail = match failure {
                crate::cmd::GitFailure::Spawn(err) => {
                    format!("{label}: failed to spawn git: {err}")
                }
                crate::cmd::GitFailure::Exit { stderr } => format!("{label} failed: {stderr}"),
            };
            Diagnostic::new("git-command-failed", project, branch, detail)
        },
    )
}

fn git_output<I, S>(
    cwd: &Path, args: I, project: &RegistryProject, branch: Option<&str>,
) -> Result<String, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output =
        crate::cmd::git_checked(&crate::cmd::real_cmd, Some(cwd), args).map_err(|failure| {
            let detail = match failure {
                crate::cmd::GitFailure::Spawn(err) => format!("failed to spawn git: {err}"),
                crate::cmd::GitFailure::Exit { stderr } => stderr,
            };
            Diagnostic::new("git-command-failed", project, branch, detail)
        })?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err(Diagnostic::new(
            "git-command-failed",
            project,
            branch,
            "git returned no output".to_string(),
        ));
    }
    Ok(text)
}

fn git_output_optional<I, S>(cwd: &Path, args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = crate::cmd::git(&crate::cmd::real_cmd, Some(cwd), args).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn git_success<I, S>(cwd: &Path, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    crate::cmd::git(&crate::cmd::real_cmd, Some(cwd), args)
        .is_ok_and(|output| output.status.success())
}
