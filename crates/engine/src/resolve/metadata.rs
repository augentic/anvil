//! Adapter metadata values and component-sidecar caching.
//!
//! Dispatch runs through an explicitly supplied [`Runner`] — never
//! process-global state; answers cache against the component SHA-256.

use std::path::{Path, PathBuf};

use emery_error::Error;
use serde::{Deserialize, Serialize};

use super::core::{AdapterLocation, Axis};
use super::routed::RoutedId;

/// A source adapter's metadata answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Metadata {
    /// Optional host-CLI compatibility floor (`emery-floor`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emery_floor: Option<String>,
}

/// One metadata dispatch by axis and routed adapter id.
#[derive(Debug)]
pub struct Request<'a> {
    /// The axis interface to invoke `metadata` on.
    pub axis: Axis,
    /// Exact routed adapter id (`<axis>:<name>[@<version>]`) — the id
    /// implied by the resolved selector: versioned for a package pin,
    /// unversioned for a cache-backed selector.
    pub adapter_id: &'a str,
}

/// Deployment-supplied metadata dispatcher.
pub type Runner = fn(&Request<'_>) -> Result<Metadata, Error>;

/// The deployed metadata dispatcher: the `emery:adapter/source` WIT
/// import, routed to the exporting guest by the request's adapter id.
///
/// # Errors
///
/// The target axis is deleted from the deployment (ADR-0008): a
/// target-axis metadata request fails typed instead of dispatching.
#[cfg(target_arch = "wasm32")]
pub fn deployed(request: &Request<'_>) -> Result<Metadata, Error> {
    match request.axis {
        Axis::Source => {
            let record = emery_adapter::source::import::metadata(request.adapter_id);
            Ok(Metadata {
                emery_floor: record.emery_floor,
            })
        }
        Axis::Target => Err(Error::Diag {
            code: "adapter-axis-removed",
            detail: format!(
                "the target adapter axis is deleted (ADR-0008); `{}` cannot be resolved",
                request.adapter_id
            ),
        }),
    }
}

/// Native builds have no adapter seam (ADR-0002): dispatch refuses
/// typed.
///
/// # Errors
///
/// Always `adapter-metadata-unsupported`.
#[cfg(not(target_arch = "wasm32"))]
pub fn deployed(request: &Request<'_>) -> Result<Metadata, Error> {
    Err(Error::Diag {
        code: "adapter-metadata-unsupported",
        detail: format!(
            "adapter `{}`: metadata dispatches over the component seam; the native path is \
             deleted (ADR-0002)",
            request.adapter_id
        ),
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataCache {
    digest: String,
    metadata: Metadata,
}

/// Sidecar path for a component file.
#[must_use]
pub(crate) fn metadata_cache_path(component: &Path) -> PathBuf {
    let mut file_name = component.file_name().map_or_else(Default::default, ToOwned::to_owned);
    file_name.push(".metadata.json");
    component.with_file_name(file_name)
}

/// Dispatch metadata by routed id, with no component file access and
/// no sidecar cache.
///
/// Dispatch happens *before* any component file is visible on the
/// caller's side of the seam — the host resolver faults the component
/// in during this dispatch, so a cold store resolves without a
/// guest-visible entry. No file means no digest key, so no cache applies.
pub(super) fn dispatch(
    runner: Runner, axis: Axis, name: &str, version: Option<&semver::Version>,
) -> Result<Metadata, Error> {
    let adapter_id = RoutedId::new(axis, name, version.cloned()).to_string();
    runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })
}

/// Load component metadata through `runner`, honoring the digest cache.
///
/// The dispatch id is the identity the selector implies: unversioned
/// (`<axis>:<name>`) for the cache-backed resolves this leg serves
/// (package pins dispatch through [`dispatch`] instead).
pub(super) fn load(
    runner: Runner, location: &AdapterLocation, axis: Axis, name: &str,
    version: Option<&semver::Version>,
) -> Result<Metadata, Error> {
    let component = location.path();
    let digest = emery_diagnostics::cache::file_content_digest(component);
    let cache_path = metadata_cache_path(component);
    if let Some(answer) = read_cache(&cache_path, &digest) {
        return Ok(answer);
    }

    let adapter_id = RoutedId::new(axis, name, version.cloned()).to_string();
    let answer = runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })?;
    write_cache(&cache_path, &digest, &answer);
    Ok(answer)
}

fn read_cache(cache_path: &Path, digest: &str) -> Option<Metadata> {
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let cache: MetadataCache = serde_json::from_str(&raw).ok()?;
    (cache.digest == digest).then_some(cache.metadata)
}

fn write_cache(cache_path: &Path, digest: &str, answer: &Metadata) {
    let cache = MetadataCache {
        digest: digest.to_string(),
        metadata: answer.clone(),
    };
    if let Ok(body) = serde_json::to_string_pretty(&cache) {
        drop(std::fs::write(cache_path, body));
    }
}
