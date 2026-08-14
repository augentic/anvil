//! Target-resolution policy: which adapter identity serves a slice.
//!
//! Fresh resolution starts from the project's declared target; once a
//! slice exists, its recorded `metadata.yaml` target is authoritative.

use error::Error;

use crate::adapter::{AdapterSelector, ResolvedTarget, Resolver};
use crate::config::{Layout, ProjectConfig};
use crate::handler::ExecutionPaths;
use crate::plan::Entry;
use crate::slice::SliceMetadata;

/// Resolve the project's declared target adapter.
///
/// # Errors
///
/// `project-no-adapter` when `project.yaml` omits `adapter`;
/// propagates adapter-resolution failures.
pub fn project_adapter(
    resolver: &impl Resolver, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Result<ResolvedTarget, Error> {
    let Some(adapter_value) = config.adapter.as_deref() else {
        return Err(Error::Diag {
            code: "project-no-adapter",
            detail: "this project has no adapter declared; per-target operations require a \
                     `project.yaml` adapter binding"
                .to_string(),
        });
    };
    resolver.resolve_target(&AdapterSelector::parse(adapter_value)?, paths)
}

/// Fresh policy: resolve `$TARGET` for a slice that does not exist yet
/// from `plan.yaml.targets[entry.target]`. `phase` names the caller's
/// verb in the operator hint (`refining`, `executing`).
///
/// # Errors
///
/// `slice-create-target-missing` when the named target is absent from
/// the plan; propagates plan-load and adapter-resolution failures.
pub fn fresh(
    resolver: &impl Resolver, paths: &ExecutionPaths, entry: &Entry, slice: &str, phase: &str,
) -> Result<String, Error> {
    let layout = paths.layout();
    let plan = crate::plan::Plan::load(&layout.plan_path())?;
    let binding = plan.target(&entry.target).map_err(|err| Error::Diag {
        code: "slice-create-target-missing",
        detail: format!(
            "no target resolved for slice `{slice}`: {err}; bind `{slice}` to a key in \
             plan.yaml.targets before {phase}"
        ),
    })?;
    let bound =
        resolver.resolve_target(&binding.adapter.selector(), paths).map_err(|err| Error::Diag {
            code: "slice-create-target-missing",
            detail: format!(
                "no target resolved for slice `{slice}`: {err}; pin `{pin}` must resolve before \
                 {phase}",
                pin = binding.adapter
            ),
        })?;
    Ok(crate::identity::target_ref(&bound.manifest.name, bound.manifest.version.as_ref()))
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

/// Best-effort advance policy: the advisory `$TARGET` for a freshly
/// advanced entry. `None` when the plan cannot resolve the bound
/// target — the build phase re-resolves before use.
pub fn best_effort_advance(
    resolver: &impl Resolver, _config: &ProjectConfig, paths: &ExecutionPaths, entry: &Entry,
) -> Option<String> {
    fresh(resolver, paths, entry, &entry.name, "advancing").ok()
}
