//! Slot adapter provisioning: mirror the workspace's project component
//! cache into a synced slot's component cache so slot-side bare-name
//! resolution stays project-local (workflow §"Resolver and cache"). See
//! [DECISIONS.md §"Slot adapter provisioning via workspace sync"].
//!
//! Post-RFC-64 an adapter is one `.wasm` component: pinned identities
//! resolve from the *global* content-addressed store (shared across
//! projects, nothing to mirror) and development bare names resolve the
//! release build live. The only workspace-owned state a slot cannot
//! reach on its own is the workspace's mirrored local components at
//! `<ws-cache>/components/*.wasm`, so that is all the mirror copies.
//!
//! [DECISIONS.md §"Slot adapter provisioning via workspace sync"]: ../../../../../DECISIONS.md#slot-adapter-provisioning-via-workspace-sync

use std::path::Path;

use specify_error::Error;
use specify_schema::cache::project_cache_dir;

/// Mirror the workspace's component cache into `slot`'s component
/// cache (keyed by the slot path, out-of-tree).
///
/// Per-file copy-over: every workspace-owned component is refreshed on
/// re-sync; slot cache entries the workspace does not own are never
/// pruned. A no-op when the workspace has no component cache.
///
/// # Errors
///
/// `workspace-adapter-mirror-failed` on any filesystem failure.
pub(super) fn mirror_adapters(workspace_dir: &Path, slot: &Path) -> Result<(), Error> {
    let source = project_cache_dir(workspace_dir).join("components");
    let Ok(entries) = std::fs::read_dir(&source) else {
        return Ok(());
    };
    let dest = project_cache_dir(slot).join("components");
    std::fs::create_dir_all(&dest).map_err(|err| mirror_error("create", &dest, &err))?;
    for entry in entries {
        let entry = entry.map_err(|err| mirror_error("read", &source, &err))?;
        let from = entry.path();
        if !from.is_file() {
            continue;
        }
        let to = dest.join(entry.file_name());
        std::fs::copy(&from, &to).map_err(|err| mirror_error("copy", &from, &err))?;
    }
    Ok(())
}

fn mirror_error(op: &str, path: &Path, err: &std::io::Error) -> Error {
    Error::Diag {
        code: "workspace-adapter-mirror-failed",
        detail: format!("failed to {op} {}: {err}", path.display()),
    }
}
