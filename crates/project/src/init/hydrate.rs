//! Registry hydration — install a pinned adapter package into the
//! global single-file store.
//!
//! A pinned package reference that misses the store is fetched from
//! the registry configured in `.specify/wasm-pkg.toml` and installed
//! as `<store-root>/<name>@<version>.wasm` plus its digest `.meta`
//! sidecar, then verified after write. Everything here is
//! deterministic; only the byte transport crosses the
//! [`Hydrator`] seam.
//!
//! The v1 wire protocol is a plain HTTP GET against a documented
//! registry layout: `<base>/adapters/<namespace>/<name>@<version>.wasm`,
//! where `<base>` is `https://<registry>` unless the configured
//! registry value already carries a scheme.

use std::fs;
use std::path::Path;

use artifacts::atomic::bytes_write;
use diagnostics::cache::{
    adapter_store_entry, adapter_store_root, file_content_digest, verify_store_entry,
    write_store_meta,
};
use error::Error;
use serde::Deserialize;

use super::adapter_uri::{PinnedPackage, pinned_package};
use crate::adapter::Hydrator;
use crate::config::Layout;

/// Hydrate the pinned package named by `value` (an `<adapter>`
/// argument or recorded `project.yaml.adapter`) when it misses the
/// global store.
///
/// Returns the installed `<name>@<version>` identity when this call
/// fetched it, `None` when `value` carries no pin or the store already
/// holds the entry.
///
/// # Errors
///
/// - `adapter-hydrate-failed` when the registry fetch fails.
/// - `adapter-digest-mismatch` when the entry fails verify-after-write.
/// - filesystem errors from the store write.
pub(super) async fn hydrate(
    hydrator: &impl Hydrator, project_dir: &Path, value: &str,
) -> Result<Option<String>, Error> {
    let Some(package) = pinned_package(value) else {
        return Ok(None);
    };
    let version = package.version.to_string();
    let entry = adapter_store_entry(&package.name, &version);
    if entry.is_file() {
        return Ok(None);
    }

    let url = package_url(project_dir, &package);
    let bytes = hydrator.fetch(&url).await.map_err(|err| Error::Diag {
        code: "adapter-hydrate-failed",
        detail: format!(
            "failed to hydrate `{}:{}@{version}` from {url}: {err}",
            package.namespace, package.name
        ),
    })?;

    fs::create_dir_all(adapter_store_root())?;
    bytes_write(&entry, &bytes)?;
    let digest = file_content_digest(&entry);
    write_store_meta(&package.name, &version, &digest, None)?;
    verify_store_entry(&package.name, &version).map_err(|mismatch| Error::Diag {
        code: "adapter-digest-mismatch",
        detail: format!(
            "store entry {} failed verify-after-write: recorded {} but read back {}",
            entry.display(),
            mismatch.recorded,
            mismatch.actual
        ),
    })?;
    Ok(Some(format!("{}@{version}", package.name)))
}

/// The registry fetch URL for one pinned package, from the project's
/// wasm-pkg configuration.
fn package_url(project_dir: &Path, package: &PinnedPackage) -> String {
    let registry = registry_for(project_dir, &package.namespace);
    let base = if registry.contains("://") { registry } else { format!("https://{registry}") };
    format!(
        "{}/adapters/{}/{}@{}.wasm",
        base.trim_end_matches('/'),
        package.namespace,
        package.name,
        package.version
    )
}

/// Registry authority the compiled default and a fresh scaffold agree
/// on; `.specify/wasm-pkg.toml` overrides it per project.
const DEFAULT_REGISTRY: &str = "augentic.io";

/// The subset of `.specify/wasm-pkg.toml` hydration reads. Mirrors the
/// wasm-pkg config shape so `wkg --config .specify/wasm-pkg.toml` and
/// the fetch leg agree on namespace routing.
#[derive(Debug, Default, Deserialize)]
struct WasmPkgConfig {
    #[serde(default)]
    default_registry: Option<String>,
    #[serde(default)]
    namespace_registries: std::collections::BTreeMap<String, String>,
}

/// Resolve the registry for `namespace`: the project's
/// `namespace_registries.<namespace>`, then its `default_registry`,
/// then the compiled default. A missing or unparseable config file
/// falls through to the default rather than failing — a fresh init
/// has not written the file yet.
fn registry_for(project_dir: &Path, namespace: &str) -> String {
    let path = Layout::new(project_dir).specify_dir().join(super::WASM_PKG_CONFIG_FILENAME);
    let config: WasmPkgConfig =
        fs::read_to_string(path).ok().and_then(|raw| toml::from_str(&raw).ok()).unwrap_or_default();
    config
        .namespace_registries
        .get(namespace)
        .cloned()
        .or(config.default_registry)
        .unwrap_or_else(|| DEFAULT_REGISTRY.to_string())
}
