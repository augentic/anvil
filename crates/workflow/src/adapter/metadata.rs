//! Adapter metadata resolution.
//!
//! An adapter's non-identity metadata — the host-CLI compatibility
//! floor, a target's declared build inputs and platforms capability —
//! lives in the component's own deterministic `metadata` export, not in
//! an on-disk manifest. The resolver obtains it through this module:
//!
//! - **Dispatch** runs through a process-global [`Runner`] seam
//!   (`workflow` stays wasmtime-free); the specify guest shim registers
//!   its runner at startup, routing each request through the
//!   deployment's WIT `source` / `target` imports by adapter id. An
//!   unregistered runner is the typed `adapter-metadata-unavailable`
//!   failure.
//! - **Caching** keys the answer on the component file's SHA-256: the
//!   answer is persisted as a `<component>.metadata.json` sidecar and
//!   reused while the recorded digest still matches the file, so a
//!   store entry is read once per install and a development build
//!   once per rebuild.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use error::Error;
use serde::{Deserialize, Serialize};

use super::core::{AdapterLocation, Axis, BuildInputDeclaration, PlatformsCapability};

/// The unified metadata answer across both axes: the WIT `metadata`
/// record projected onto one serde shape. A source answer carries only
/// `specify-floor`; the target-only fields default empty/absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Metadata {
    /// Optional host-CLI compatibility floor (`specify-floor`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specify_floor: Option<String>,
    /// Target-declared build inputs; empty for source adapters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<BuildInputDeclaration>,
    /// Target platforms capability; absent for source adapters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<PlatformsCapability>,
}

/// One metadata dispatch: resolve the answer for the component at
/// `component` by invoking `metadata` on the `axis` interface with
/// `adapter_id`.
#[derive(Debug)]
pub struct MetadataRequest<'a> {
    /// The adapter component file.
    pub component: &'a Path,
    /// The axis interface to invoke `metadata` on.
    pub axis: Axis,
    /// The routed adapter id passed as the call's first argument
    /// (`<axis>:<name>`).
    pub adapter_id: &'a str,
}

/// The process-global metadata dispatcher the guest shim registers.
pub type Runner = fn(&MetadataRequest<'_>) -> Result<Metadata, Error>;

static RUNNER: OnceLock<Runner> = OnceLock::new();

/// Register the process-global metadata dispatcher. First registration
/// wins; later calls are no-ops (the guest shim registers exactly one
/// runner at startup, and tests may register a stub).
pub fn register(runner: Runner) {
    let _ = RUNNER.set(runner);
}

/// Digest-keyed sidecar persisted beside the component.
#[derive(Debug, Serialize, Deserialize)]
struct MetadataCache {
    /// `sha256:<hex>` of the component file the answer was produced
    /// from.
    digest: String,
    /// The cached answer.
    metadata: Metadata,
}

/// The sidecar path for a component file —
/// `<component>.metadata.json` (e.g. `omnia@1.0.0.wasm.metadata.json`).
#[must_use]
pub fn metadata_cache_path(component: &Path) -> PathBuf {
    let mut file_name = component.file_name().map_or_else(Default::default, ToOwned::to_owned);
    file_name.push(".metadata.json");
    component.with_file_name(file_name)
}

/// Resolve the metadata answer for a located component.
///
/// Returns the digest-valid cached sidecar when present, else
/// dispatches through the registered runner and records the sidecar
/// (best-effort — a read-only sidecar location degrades to per-resolve
/// dispatch, not an error).
///
/// # Errors
///
/// - `adapter-metadata-unavailable` — no runner is registered in this
///   process.
/// - Any error from the runner itself (`adapter-metadata-failed`,
///   `adapter-axis-mismatch`).
pub fn metadata(location: &AdapterLocation, axis: Axis, name: &str) -> Result<Metadata, Error> {
    let component = location.path();
    let digest = schema::cache::file_content_digest(component);
    let cache_path = metadata_cache_path(component);
    if let Some(answer) = read_cache(&cache_path, &digest) {
        return Ok(answer);
    }

    let runner = RUNNER.get().ok_or_else(|| Error::Diag {
        code: "adapter-metadata-unavailable",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) at {} needs a host-side metadata dispatch, but no \
             metadata runner is registered in this process",
            component.display(),
        ),
    })?;
    let adapter_id = format!("{}:{name}", axis_prefix(axis));
    let request = MetadataRequest {
        component,
        axis,
        adapter_id: &adapter_id,
    };
    let answer = runner(&request)?;
    write_cache(&cache_path, &digest, &answer);
    Ok(answer)
}

/// The `<axis>` half of the routed adapter id (`source:<name>` /
/// `target:<name>`) — the singular form, matching the deployment guest
/// ids and the workflow guest's dispatch routing.
const fn axis_prefix(axis: Axis) -> &'static str {
    match axis {
        Axis::Source => "source",
        Axis::Target => "target",
    }
}

fn read_cache(cache_path: &Path, digest: &str) -> Option<Metadata> {
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let cache: MetadataCache = serde_json::from_str(&raw).ok()?;
    (cache.digest == digest).then_some(cache.metadata)
}

/// Best-effort sidecar write: a failure (read-only parent, races) only
/// costs a re-dispatch on the next resolve.
fn write_cache(cache_path: &Path, digest: &str, answer: &Metadata) {
    let cache = MetadataCache {
        digest: digest.to_string(),
        metadata: answer.clone(),
    };
    if let Ok(body) = serde_json::to_string_pretty(&cache) {
        drop(std::fs::write(cache_path, body));
    }
}
