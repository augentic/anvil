//! In-process shell presence detection for Vectis-bound projects.
//! Non-Vectis targets return an empty missing set without invoking
//! shell heuristics (RFC-46 Phase 0).

#[cfg(test)]
#[path = "detect/tests.rs"]
mod tests;

use std::collections::HashSet;
use std::path::Path;

use specify_error::{Error, Result};
use specify_vectis_shell_detect::missing_shell_platforms;

use crate::Platform;
use crate::adapter::TargetAdapter;
use crate::config::ProjectConfig;
use crate::init::adapter_ref_from_value;

const VECTIS_ADAPTER: &str = "vectis";

/// Return declared-but-absent platforms for a Vectis-bound project via
/// in-process shell heuristics.
///
/// When the project's target adapter is not `vectis`, returns an empty
/// vector without scanning the tree (Omnia and other targets are
/// unaffected). The result preserves `declared` order and is filtered to
/// platforms present in both the caller's `declared` set and the missing
/// set from [`missing_shell_platforms`].
///
/// # Errors
///
/// Propagates [`ProjectConfig::load`] and adapter resolution failures.
pub fn vectis_missing_platforms(
    project_dir: &Path, declared: &[Platform],
) -> Result<Vec<Platform>, Error> {
    if declared.is_empty() || !is_vectis_bound(project_dir)? {
        return Ok(Vec::new());
    }

    let declared_strs: Vec<String> = declared.iter().map(ToString::to_string).collect();
    let declared_refs: Vec<&str> = declared_strs.iter().map(String::as_str).collect();
    let missing_strs = missing_shell_platforms(project_dir, &declared_refs);

    // `missing_shell_platforms` only emits supported shell platform names.
    let missing_set: HashSet<Platform> =
        missing_strs.into_iter().filter_map(|name| name.parse().ok()).collect();

    Ok(declared.iter().copied().filter(|p| missing_set.contains(p)).collect())
}

fn is_vectis_bound(project_dir: &Path) -> Result<bool, Error> {
    let config = ProjectConfig::load(project_dir)?;
    let Some(adapter_value) = config.adapter.as_deref() else {
        return Ok(false);
    };
    let adapter_ref = adapter_ref_from_value(adapter_value);
    let resolved = TargetAdapter::resolve(&adapter_ref, project_dir)?;
    Ok(resolved.manifest.name == VECTIS_ADAPTER)
}
