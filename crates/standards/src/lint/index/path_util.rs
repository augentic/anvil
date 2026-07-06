//! Shared path-rendering helpers for the framework-profile indexers.
//!
//! The symlink and agent-teams indexers both emit `/`-normalised,
//! project-relative path strings. The normalisation rule lives here so
//! the two indexers share one definition.

use std::path::{MAIN_SEPARATOR, Path};

/// Render `p` as a `/`-separated string, or `None` when the path is not
/// valid UTF-8. On `/`-native platforms this is a borrow-free passthrough.
pub(super) fn render(p: &Path) -> Option<String> {
    let s = p.to_str()?;
    if MAIN_SEPARATOR == '/' { Some(s.to_owned()) } else { Some(s.replace(MAIN_SEPARATOR, "/")) }
}

/// Canonicalise `link` and return its [`render`]ed project-relative path
/// when the endpoint resolves under `project_dir`. Off-tree or
/// unreadable targets yield `None`.
pub(super) fn canonicalise_into_project(link: &Path, project_dir: &Path) -> Option<String> {
    let canon_link = std::fs::canonicalize(link).ok()?;
    let canon_project = std::fs::canonicalize(project_dir).ok()?;
    let relative = canon_link.strip_prefix(&canon_project).ok()?;
    render(relative)
}
