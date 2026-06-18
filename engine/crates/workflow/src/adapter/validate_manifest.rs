//! Adapter-manifest schema validation and the cross-axis collision probe.
//!
//! The schema gate runs the shared `adapter.schema.json` then the
//! axis-specific `source.schema.json` / `target.schema.json`. The
//! collision probe enforces the cross-axis name-uniqueness invariant
//! (see [DECISIONS.md §"Adapter name uniqueness"]).
//!
//! [DECISIONS.md §"Adapter name uniqueness"]: ../../../../DECISIONS.md#adapter-name-uniqueness

use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_schema::{
    ADAPTER_JSON_SCHEMA, SOURCE_JSON_SCHEMA, TARGET_JSON_SCHEMA, ValidationStatus,
};

use super::core::{ADAPTER_FILENAME, Axis, adapter_axis_dir, cache_dir};
use crate::schema::validate_value_cached;

/// Probe the (axis, name) pair for an `adapter.yaml` under both the
/// manifest cache and the in-repo tree. Returns the first hit — the
/// cache wins over `local`, mirroring [`super::resolve`]'s probe order
/// — or `None` when neither location declares a manifest. A bare
/// directory without `adapter.yaml` is treated as absent so the
/// cross-axis collision check fires only on declared manifests, not
/// stale empty cache slots.
pub(super) fn sibling_manifest_path(axis: Axis, name: &str, project_dir: &Path) -> Option<PathBuf> {
    let cached = cache_dir(project_dir, axis, name);
    if cached.join(ADAPTER_FILENAME).is_file() {
        return Some(cached);
    }
    let local = adapter_axis_dir(project_dir, axis).join(name);
    if local.join(ADAPTER_FILENAME).is_file() {
        return Some(local);
    }
    None
}

pub(super) fn axis_collision_error(
    name: &str, located_axis: Axis, located_path: &Path, sibling_path: &Path,
) -> Error {
    let opposite = located_axis.opposite();
    Error::validation_failed(
        "adapter-name-axis-collision",
        format!("adapter name `{name}` is unique across axes"),
        format!(
            "adapter name `{name}` is declared under both `adapters/sources/` and `adapters/targets/` (or the equivalent out-of-tree `<project-cache>/manifests/{{sources,targets}}/<name>/` mirror); names must be unique across axes (axis `{located_axis}`: {}; axis `{opposite}`: {})",
            located_path.display(),
            sibling_path.display(),
        ),
    )
}

/// Validate that installing or resolving `name` on `axis` does not
/// collide with an existing declaration on the opposite axis.
///
/// Used at `specify init` time (with `axis = Axis::Target`, since
/// `init` only caches target adapters) before the per-axis manifest
/// cache directory at
/// `<project-cache>/manifests/{sources,targets}/<name>/` is rewritten,
/// so the operator hits a clear collision diagnostic ahead of the
/// downstream `TargetAdapter::resolve` call. The same invariant fires
/// inside the private `locate_axis` helper used by
/// [`super::core::SourceAdapter::resolve`] /
/// [`super::core::TargetAdapter::resolve`]; this one-sided helper is
/// the cheap "the side I'm about to install on may not yet exist on
/// disk" variant.
///
/// # Errors
///
/// Returns [`Error::Validation`] with the kebab discriminant
/// `adapter-name-axis-collision` when the opposite axis already
/// declares a manifest for `name`. The error body names both axes.
pub fn check_axis_unique_for_name(axis: Axis, name: &str, project_dir: &Path) -> Result<(), Error> {
    let opposite = axis.opposite();
    let Some(sibling) = sibling_manifest_path(opposite, name, project_dir) else {
        return Ok(());
    };
    // The error body must name both axes; pass through the
    // axis-being-installed as the "located" axis even when no
    // manifest exists on disk for it yet — the diagnostic prose is
    // about the *name* clash, not which side resolved first.
    let here = adapter_axis_dir(project_dir, axis).join(name);
    Err(axis_collision_error(name, axis, &here, &sibling))
}

pub(super) fn validate_schema(
    axis: Axis, manifest_path: &Path, instance: &serde_json::Value,
) -> Result<(), Error> {
    // Shape gate first — catches violations both schemas share.
    run_schema(ADAPTER_JSON_SCHEMA, manifest_path, instance, "adapter")?;
    // Axis-specific refinement (operation set + axis literal).
    let (schema, label) = match axis {
        Axis::Source => (SOURCE_JSON_SCHEMA, "source"),
        Axis::Target => (TARGET_JSON_SCHEMA, "target"),
    };
    run_schema(schema, manifest_path, instance, label)
}

fn run_schema(
    schema_source: &'static str, manifest_path: &Path, instance: &serde_json::Value, label: &str,
) -> Result<(), Error> {
    let rule = format!("{} conforms to embedded {label} schema", manifest_path.display());
    for summary in validate_value_cached(instance, schema_source, "adapter-schema-violation", &rule)
    {
        if summary.status == ValidationStatus::Fail {
            return Err(Error::Diag {
                code: "adapter-schema-violation",
                detail: format!(
                    "{} violates {label} schema: {}",
                    manifest_path.display(),
                    summary.detail.unwrap_or_default()
                ),
            });
        }
    }
    Ok(())
}
