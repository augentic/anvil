//! Symlink fact recorder per the `WorkspaceModel` entity families and
//! the file scan contract.
//!
//! Called by the consumer and framework walkers for each entry whose
//! `file_type()` is a symlink. Under [`FollowMode::Record`] (the
//! consumer policy from the file scan contract) the link is visited
//! but the underlying target is never traversed and only the
//! `(path, target, broken)` triple is recorded. Under
//! [`FollowMode::Follow`] (the framework policy per the standards-layer
//! contract §F1) the recorder additionally canonicalises the link and
//! records the project-relative endpoint in [`Symlink::resolved_target`]
//! so review-team-protocol drift surfaces in the model. Cycle
//! detection lives in the caller — the recorder is stateless.

use std::path::Path;

use crate::lint::Symlink;

/// Symlink-follow policy passed to [`record`].
///
/// `Record` is the consumer policy: capture the path / target /
/// broken triple but never traverse through the link. `Follow` is
/// the framework policy per §F1: capture the same triple and
/// additionally canonicalise the link so the resolved endpoint
/// surfaces in [`Symlink::resolved_target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowMode {
    /// Record the link without resolving the canonical target.
    /// The project profile's default.
    Record,
    /// Record the link and resolve the canonical target so both
    /// endpoints appear in the model. The framework profile's
    /// default per §F1.
    Follow,
}

/// Build a [`Symlink`] fact from an on-disk symlink entry.
///
/// Returns `None` when:
/// - the entry sits outside `project_dir` (the strip-prefix fails),
/// - `read_link` fails (the link node disappeared between walk and
///   read), or
/// - either path cannot be rendered as UTF-8.
///
/// `broken` is computed via `Path::exists()`, which dereferences the
/// symlink — a missing or self-referencing target yields `true`.
/// Under [`FollowMode::Follow`] `resolved_target` is populated when
/// the canonical endpoint resolves under `project_dir`; targets
/// pointing outside the tree leave the field absent so consumers can
/// distinguish on-tree from off-tree endpoints.
#[must_use]
pub fn record(path: &Path, project_dir: &Path, mode: FollowMode) -> Option<Symlink> {
    let relative = path.strip_prefix(project_dir).ok()?;
    let path_str = super::path_util::render(relative)?;
    let target = std::fs::read_link(path).ok()?;
    let target_str = super::path_util::render(&target)?;
    let broken = !path.exists();
    let resolved_target = match mode {
        FollowMode::Record => None,
        FollowMode::Follow => super::path_util::canonicalise_into_project(path, project_dir),
    };
    Some(Symlink {
        path: path_str,
        target: target_str,
        broken,
        resolved_target,
    })
}
