//! Pure derivation helpers for [`super::prepare`] — target branch
//! name, slot path, remote default head, and the safe-dirty allow-list
//! used for slice resume.

use std::path::{Component, Path, PathBuf};

use specify_error::is_kebab;

use super::{Diagnostic, git_output, git_output_optional};
use crate::registry::catalog::RegistryProject;

const ORIGIN_HEAD_UNRESOLVED: &str = "origin-head-unresolved";

pub(super) fn target_branch(
    project: &RegistryProject, change_name: &str,
) -> Result<String, Diagnostic> {
    let branch = format!("specify/{change_name}");
    if is_kebab(change_name) && !change_name.contains('/') {
        return Ok(branch);
    }
    Err(Diagnostic::new(
        "branch-pattern-mismatch",
        project,
        Some(&branch),
        format!(
            "branch `{branch}` is outside the exact `specify/<change-name>` pattern; \
             change names must be kebab-case without slashes"
        ),
    ))
}

pub(super) fn project_worktree_path(project_dir: &Path, project: &RegistryProject) -> PathBuf {
    let workspace_slot = project_dir.join("workspace").join(&project.name);
    if !project.is_local() || workspace_slot.exists() {
        return workspace_slot;
    }
    if project.url == "." { project_dir.to_path_buf() } else { project_dir.join(&project.url) }
}

pub(super) fn refresh_origin_head(slot_path: &Path) {
    drop(crate::cmd::git(
        &crate::cmd::real_cmd,
        Some(slot_path),
        ["remote", "set-head", "origin", "--auto"],
    ));
}

pub(super) fn resolve_origin_head(
    slot_path: &Path, project: &RegistryProject, branch: &str,
) -> Result<String, Diagnostic> {
    let symbolic =
        git_output_optional(slot_path, ["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"]);
    let Some(base_ref) = symbolic else {
        return Err(Diagnostic::new(
            ORIGIN_HEAD_UNRESOLVED,
            project,
            Some(branch),
            "origin-head-unresolved: could not resolve `origin/HEAD` after fetch; \
             refusing to guess a default branch",
        ));
    };
    git_output(slot_path, ["rev-parse", "--verify", "origin/HEAD^{commit}"], project, Some(branch))
        .map(|_| base_ref)
        .map_err(|_err| {
            Diagnostic::new(
                ORIGIN_HEAD_UNRESOLVED,
                project,
                Some(branch),
                "origin-head-unresolved: `origin/HEAD` is not a commit; refusing to guess a default branch",
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AllowedPath {
    pub(super) rel: String,
    pub(super) display: String,
    pub(super) is_dir: bool,
}

impl AllowedPath {
    pub(super) fn matches(&self, path: &str) -> bool {
        path == self.rel || (self.is_dir && path.starts_with(&format!("{}/", self.rel)))
    }
}

pub(super) fn allowed_paths(
    slot_path: &Path, change_name: &str, source_paths: &[PathBuf], output_paths: &[PathBuf],
) -> Vec<AllowedPath> {
    let mut allowed = vec![
        AllowedPath {
            rel: format!(".specify/slices/{change_name}"),
            display: format!(".specify/slices/{change_name}/"),
            is_dir: true,
        },
        AllowedPath {
            rel: ".specify/specs".to_string(),
            display: ".specify/specs/".to_string(),
            is_dir: true,
        },
        AllowedPath {
            rel: ".specify/archive".to_string(),
            display: ".specify/archive/".to_string(),
            is_dir: true,
        },
        AllowedPath {
            rel: "crates".to_string(),
            display: "crates/".to_string(),
            is_dir: true,
        },
        AllowedPath {
            rel: "contracts".to_string(),
            display: "contracts/".to_string(),
            is_dir: true,
        },
        AllowedPath {
            rel: "apps".to_string(),
            display: "apps/".to_string(),
            is_dir: true,
        },
    ];

    for path in source_paths.iter().chain(output_paths) {
        if let Some((rel, is_dir)) = relative_allowed_path(slot_path, path) {
            let display = if is_dir { format!("{rel}/") } else { rel.clone() };
            allowed.push(AllowedPath { rel, display, is_dir });
        }
    }

    allowed.sort_by(|a, b| a.rel.cmp(&b.rel));
    allowed.dedup_by(|a, b| a.rel == b.rel && a.is_dir == b.is_dir);
    allowed
}

fn relative_allowed_path(slot_path: &Path, input: &Path) -> Option<(String, bool)> {
    let candidate = if input.is_absolute() {
        let canonical_slot = std::fs::canonicalize(slot_path).ok();
        input
            .strip_prefix(slot_path)
            .ok()
            .or_else(|| canonical_slot.as_deref().and_then(|slot| input.strip_prefix(slot).ok()))?
            .to_path_buf()
    } else {
        input.to_path_buf()
    };
    let rel = path_to_slash(&candidate)?;
    if rel.is_empty() {
        return None;
    }
    let is_dir = slot_path.join(&candidate).is_dir();
    Some((rel, is_dir))
}

fn path_to_slash(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}
