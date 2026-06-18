//! Slot adapter provisioning: mirror the workspace's adapter
//! set into a synced slot's manifest cache so slot-side resolution
//! stays project-local (workflow §"Resolver and cache"). See
//! [DECISIONS.md §"Slot adapter provisioning via workspace sync"].
//!
//! [DECISIONS.md §"Slot adapter provisioning via workspace sync"]: ../../../../../DECISIONS.md#slot-adapter-provisioning-via-workspace-sync

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specify_error::Error;

use crate::adapter::{ADAPTER_FILENAME, Axis, adapter_axis_dir, cache_axis_dir, cache_dir};

/// Mirror the workspace's adapters — both axes, vendored tree and
/// manifest-cache mirror alike, `tools.yaml` sidecars included — into
/// `slot`'s out-of-tree manifest cache at
/// `<slot-cache>/manifests/{sources,targets}/` (keyed by the slot path).
///
/// Per-name delete-then-copy: every workspace-owned name is refreshed
/// on re-sync; cache entries the workspace does not own (e.g. an
/// init-time greenfield adapter seed) are never pruned. A name the
/// slot vendors under its own `adapters/` tree — on either axis — is
/// skipped, so the slot copy keeps winning resolution (the loader
/// probes the cache first) and the mirror can never manufacture an
/// `adapter-name-axis-collision` in a previously healthy slot.
///
/// # Errors
///
/// `workspace-adapter-mirror-failed` on any filesystem failure.
pub(super) fn mirror_adapters(workspace_dir: &Path, slot: &Path) -> Result<(), Error> {
    for axis in [Axis::Source, Axis::Target] {
        for (name, source) in workspace_adapters(workspace_dir, axis) {
            if slot_vendors(slot, &name) {
                continue;
            }
            replace_dir(&source, &cache_dir(slot, axis, &name))?;
        }
    }
    Ok(())
}

/// The workspace's adapter set for one axis, name → source directory.
///
/// A name present in both probe locations copies from the manifest
/// cache, matching the loader's cache-over-vendored probe order. A
/// bare directory without `adapter.yaml` is not an adapter (same rule
/// as `adapter::validate_manifest`).
fn workspace_adapters(workspace_dir: &Path, axis: Axis) -> BTreeMap<String, PathBuf> {
    let mut adapters = BTreeMap::new();
    for root in [adapter_axis_dir(workspace_dir, axis), cache_axis_dir(workspace_dir, axis)] {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.join(ADAPTER_FILENAME).is_file() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                adapters.insert(name.to_string(), dir);
            }
        }
    }
    adapters
}

fn slot_vendors(slot: &Path, name: &str) -> bool {
    [Axis::Source, Axis::Target]
        .into_iter()
        .any(|axis| adapter_axis_dir(slot, axis).join(name).join(ADAPTER_FILENAME).is_file())
}

fn replace_dir(source: &Path, dest: &Path) -> Result<(), Error> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|err| mirror_error("remove", dest, &err))?;
    }
    copy_dir(source, dest)
}

fn copy_dir(source: &Path, dest: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dest).map_err(|err| mirror_error("create", dest, &err))?;
    let entries = std::fs::read_dir(source).map_err(|err| mirror_error("read", source, &err))?;
    for entry in entries {
        let entry = entry.map_err(|err| mirror_error("read", source, &err))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            // Follows symlinks: a brief symlinked at the workspace
            // lands as a regular file in the slot cache.
            std::fs::copy(&from, &to).map_err(|err| mirror_error("copy", &from, &err))?;
        }
    }
    Ok(())
}

fn mirror_error(op: &str, path: &Path, err: &std::io::Error) -> Error {
    Error::Diag {
        code: "workspace-adapter-mirror-failed",
        detail: format!("failed to {op} {}: {err}", path.display()),
    }
}
