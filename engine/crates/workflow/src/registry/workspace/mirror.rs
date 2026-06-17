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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

    /// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
    struct CacheGuard(Option<std::ffi::OsString>);

    impl Drop for CacheGuard {
        #[expect(unsafe_code, reason = "restore the cache-root env var pinned for the test")]
        fn drop(&mut self) {
            // SAFETY: nextest runs each test in its own process, so no
            // other thread reads or writes this env var concurrently.
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var(CACHE_ENV, value),
                    None => std::env::remove_var(CACHE_ENV),
                }
            }
        }
    }

    /// Pin the out-of-tree cache root inside `tmp` for the test's
    /// lifetime so cache writes are hermetic and auto-cleaned.
    #[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
    fn scoped_cache(tmp: &TempDir) -> CacheGuard {
        let prev = std::env::var_os(CACHE_ENV);
        // SAFETY: see `CacheGuard::drop` — single-process test isolation.
        unsafe { std::env::set_var(CACHE_ENV, tmp.path().join("project-cache")) };
        CacheGuard(prev)
    }

    fn stage_adapter(root: &Path, rel: &str, body: &str) {
        let dir = root.join(rel);
        fs::create_dir_all(&dir).expect("create adapter dir");
        fs::write(dir.join(ADAPTER_FILENAME), body).expect("write adapter.yaml");
    }

    #[test]
    fn cache_wins_over_vendored_as_copy_source() {
        // The workspace carries the same name in both probe locations;
        // the mirror copies from the cache, matching the loader.
        let tmp = TempDir::new().unwrap();
        let _cache = scoped_cache(&tmp);
        stage_adapter(tmp.path(), "adapters/sources/docs", "vendored\n");
        let cached = cache_dir(tmp.path(), Axis::Source, "docs");
        fs::create_dir_all(&cached).expect("create cached adapter dir");
        fs::write(cached.join(ADAPTER_FILENAME), "cached\n").expect("write cached adapter.yaml");

        let adapters = workspace_adapters(tmp.path(), Axis::Source);
        let source = adapters.get("docs").expect("docs adapter present");
        assert_eq!(fs::read_to_string(source.join(ADAPTER_FILENAME)).unwrap(), "cached\n");
    }

    #[test]
    fn bare_directory_is_not_an_adapter() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("adapters/sources/empty")).unwrap();

        assert!(workspace_adapters(tmp.path(), Axis::Source).is_empty());
    }

    #[test]
    fn slot_vendoring_is_cross_axis() {
        let tmp = TempDir::new().unwrap();
        stage_adapter(tmp.path(), "adapters/targets/shared", "slot copy\n");

        assert!(slot_vendors(tmp.path(), "shared"), "opposite-axis vendoring must count");
        assert!(!slot_vendors(tmp.path(), "other"));
    }

    #[test]
    fn replace_refreshes_stale_copy() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("src");
        fs::create_dir_all(source.join("briefs")).unwrap();
        fs::write(source.join(ADAPTER_FILENAME), "fresh\n").unwrap();
        fs::write(source.join("briefs/extract.md"), "# brief\n").unwrap();
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join(ADAPTER_FILENAME), "stale\n").unwrap();
        fs::write(dest.join("orphan.md"), "stale file\n").unwrap();

        replace_dir(&source, &dest).expect("replace ok");

        assert_eq!(fs::read_to_string(dest.join(ADAPTER_FILENAME)).unwrap(), "fresh\n");
        assert_eq!(fs::read_to_string(dest.join("briefs/extract.md")).unwrap(), "# brief\n");
        assert!(!dest.join("orphan.md").exists(), "delete-then-copy must drop stale files");
    }
}
