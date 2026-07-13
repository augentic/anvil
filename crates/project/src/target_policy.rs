//! Target-resolution policy: which adapter identity serves a slice at
//! each point in its life.
//!
//! - [`project_adapter`] — the project's declared target adapter, the
//!   identity every fresh resolution starts from.
//! - [`fresh`] — before a slice exists, `$TARGET` resolves from the
//!   bound project's topology.
//! - [`resumed`] — after creation, the slice's recorded
//!   `metadata.yaml` target is authoritative.
//! - [`best_effort_next`] — `plan next`'s advisory `$TARGET`
//!   projection; `None` when the topology cannot resolve (the build
//!   phase re-resolves before use).

use std::path::Path;

use error::Error;

use crate::adapter::{AdapterRef, ResolvedTarget, Resolver};
use crate::config::{Layout, ProjectConfig};
use crate::plan::{Entry, resolve_target, resolve_topology};
use crate::slice::SliceMetadata;

/// Resolve the project's declared target adapter.
///
/// Workspace projects (`workspace: true`, `adapter:` omitted) do not
/// declare an adapter, so this returns a `workspace-no-adapter`
/// diagnostic naming the workspace case rather than a stray
/// adapter-resolution error lower down the stack.
///
/// # Errors
///
/// `workspace-no-adapter` for adapter-less workspace projects;
/// propagates adapter-resolution failures.
pub fn project_adapter(
    resolver: &impl Resolver, config: &ProjectConfig, project_dir: &Path,
) -> Result<ResolvedTarget, Error> {
    let Some(adapter_value) = config.adapter.as_deref() else {
        return Err(Error::Diag {
            code: "workspace-no-adapter",
            detail: "this project has no adapter declared (workspaces do not run per-target \
                     operations); only `specify registry` and `specify plan` verbs are supported \
                     on workspaces"
                .to_string(),
        });
    };
    resolver.resolve_target(&AdapterRef::from_value(adapter_value), project_dir)
}

/// Fresh policy: resolve `$TARGET` for a slice that does not exist yet
/// from the bound project's topology. `phase` names the caller's verb
/// in the operator hint (`refining`, `executing`).
///
/// # Errors
///
/// `slice-create-target-missing` when the topology does not resolve a
/// target for `entry`; propagates config-load and topology failures.
pub fn fresh(
    resolver: &impl Resolver, layout: Layout<'_>, entry: &Entry, slice: &str, phase: &str,
) -> Result<String, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let topology = resolve_topology(resolver, &config, layout.project_dir())?;
    resolve_target(entry, &topology).map(|target| target.to_string()).map_err(|err| Error::Diag {
        code: "slice-create-target-missing",
        detail: format!(
            "no target resolved for slice `{slice}`: {err}; declare the project adapter (or fix \
             the bound project's topology) before {phase}"
        ),
    })
}

/// Resumed policy: the slice's recorded `metadata.yaml` target —
/// authoritative once the slice exists.
///
/// # Errors
///
/// Propagates the [`SliceMetadata`] load failure (absent slice tree).
pub fn resumed(layout: Layout<'_>, slice: &str) -> Result<String, Error> {
    Ok(SliceMetadata::load(&layout.slice_dir(slice))?.target)
}

/// Best-effort-next policy: `plan next`'s advisory `$TARGET` for a
/// freshly advanced entry. `None` when the topology cannot resolve —
/// the build phase re-resolves the target before use.
pub fn best_effort_next(
    resolver: &impl Resolver, config: &ProjectConfig, project_dir: &Path, entry: &Entry,
) -> Option<String> {
    resolve_topology(resolver, config, project_dir)
        .and_then(|topology| resolve_target(entry, &topology))
        .ok()
        .map(|target| target.to_string())
}
