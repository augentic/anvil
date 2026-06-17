//! Orphan-cache scan for `specify tool gc` — computes the cache
//! `<scope>/<tool>/<version>/` directories that are no longer
//! referenced by the live merged manifest.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::hash::BuildHasher;
use std::path::PathBuf;

use super::{SIDECAR_FILENAME, read_sidecar, root, scope_segment, sorted_dir_entries};
use crate::error::ExtensionError;
use crate::manifest::ExtensionScope;

/// Return version directories under `scope` not referenced by `kept`.
///
/// The keep-set tuple is `(tool-name, tool-version, source)`. The scan is
/// limited to the supplied scope segment; another project or plugin with
/// the same tool name is not considered. The returned vector is deduplicated
/// by the directory layout (one entry per `<tool>/<version>/` directory) and
/// sorted for deterministic output.
///
/// # Errors
///
/// Returns the `tool-cache-root` diagnostic when the cache root cannot
/// be selected, the `tool-resolver` diagnostic when a discovered
/// directory name is not valid UTF-8 or violates the cache-segment
/// invariants, the `tool-io` diagnostic when the scope or tool directory
/// cannot be read, and the `tool-sidecar-parse` / `tool-sidecar-schema`
/// diagnostics when an existing `meta.yaml` is malformed (a missing
/// sidecar marks the directory as unreferenced rather than erroring).
pub fn scan<S: BuildHasher>(
    scope: &ExtensionScope, kept: &HashSet<(String, String, String), S>,
) -> Result<Vec<PathBuf>, ExtensionError> {
    let scope_dir = root()?.join(scope_segment(scope)?);
    if !scope_dir.exists() {
        return Ok(Vec::new());
    }

    let mut unreferenced = Vec::new();
    for tool_entry in sorted_dir_entries(&scope_dir)? {
        if !tool_entry.path().is_dir() {
            continue;
        }
        let tool_name = file_name_string(&tool_entry.path(), "tool cache directory")?;
        for version_entry in sorted_dir_entries(&tool_entry.path())? {
            let version_dir = version_entry.path();
            if !version_dir.is_dir() {
                continue;
            }
            let version = file_name_string(&version_dir, "tool version directory")?;
            let Some(sidecar) = read_sidecar(&version_dir.join(SIDECAR_FILENAME))? else {
                unreferenced.push(version_dir);
                continue;
            };
            let key = (tool_name.clone(), version, sidecar.source);
            if !kept.contains(&key) || sidecar.scope != scope_segment(scope)? {
                unreferenced.push(version_dir);
            }
        }
    }
    unreferenced.sort();
    Ok(unreferenced)
}

fn file_name_string(path: &std::path::Path, field: &'static str) -> Result<String, ExtensionError> {
    path.file_name().and_then(OsStr::to_str).map(ToOwned::to_owned).ok_or_else(|| {
        ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "invalid tool cache segment `{}` for {field}: must be valid UTF-8",
                path.display()
            ),
        }
    })
}
