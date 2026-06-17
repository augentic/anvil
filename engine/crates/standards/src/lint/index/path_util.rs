//! Shared path-rendering helpers for the framework-profile indexers.
//!
//! The symlink and agent-teams indexers both emit `/`-normalised,
//! project-relative path strings. The normalisation rule lives here so
//! the two indexers share one definition. The adapter / brief
//! extractors share the `adapters/{sources,targets}/<adapter>/<tail>`
//! prefix split through `parse_adapter_prefix`.

use std::path::{MAIN_SEPARATOR, Path};

use crate::lint::AdapterAxis;

/// Render `p` as a `/`-separated string, or `None` when the path is not
/// valid UTF-8. On `/`-native platforms this is a borrow-free passthrough.
pub(super) fn render(p: &Path) -> Option<String> {
    let s = p.to_str()?;
    if MAIN_SEPARATOR == '/' { Some(s.to_owned()) } else { Some(s.replace(MAIN_SEPARATOR, "/")) }
}

/// Split `adapters/{sources,targets}/<adapter>/<tail>` into the
/// `(axis, adapter, tail)` triple, where `tail` is everything after the
/// adapter segment. Returns `None` for any path outside the canonical
/// `adapters/<axis>/<adapter>/…` layout or with an empty adapter
/// segment. The adapter and brief extractors apply their own tail
/// checks (`adapter.yaml`, `briefs/<op>.md`) on the returned `tail`.
pub(super) fn parse_adapter_prefix(relative: &str) -> Option<(AdapterAxis, &str, &str)> {
    let rest = relative.strip_prefix("adapters/")?;
    let (axis_str, rest) = rest.split_once('/')?;
    let axis = match axis_str {
        "sources" => AdapterAxis::Sources,
        "targets" => AdapterAxis::Targets,
        _ => return None,
    };
    let (adapter, tail) = rest.split_once('/')?;
    if adapter.is_empty() {
        return None;
    }
    Some((axis, adapter, tail))
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
