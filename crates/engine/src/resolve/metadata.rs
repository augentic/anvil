//! Adapter metadata values and component-sidecar caching.
//!
//! Dispatch runs through an explicitly supplied [`Runner`] — never
//! process-global state; answers cache against the component SHA-256.

use emery_error::Error;
use omnia_guest::BlobStore;
use serde::{Deserialize, Serialize};

use super::core::{AdapterLocation, Axis};
use super::routed::RoutedId;
use crate::handler::{ADAPTERS_CONTAINER, STORE_CONTAINER};

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
pub trait Runner: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync {}

impl<F: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync> Runner for F {}

/// The metadata runner over the provider's source-seam capability
/// ([`emery_adapter::Source`]).
///
/// The returned closure answers the source axis through the provider;
/// the target axis is deleted from the deployment (ADR-0008), so a
/// target-axis request fails typed (`adapter-axis-removed`) instead of
/// dispatching.
pub fn runner<P: emery_adapter::Source>(provider: &P) -> impl Runner + '_ {
    move |request: &Request<'_>| match request.axis {
        Axis::Source => {
            let record = provider.metadata(request.adapter_id);
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

#[derive(Debug, Serialize, Deserialize)]
struct MetadataCache {
    digest: String,
    metadata: Metadata,
}

// The component's container and object name from its resolved
// location: the sidecar cache lives beside the component it keys.
fn slot(location: &AdapterLocation) -> (&'static str, String) {
    let object = location
        .path()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match location {
        AdapterLocation::Store(_) => (STORE_CONTAINER, object),
        AdapterLocation::Cache(_) => (ADAPTERS_CONTAINER, object),
    }
}

/// Dispatch metadata by routed id, with no component file access and
/// no sidecar cache.
///
/// Dispatch happens *before* any component file is visible on the
/// caller's side of the seam — the host resolver faults the component
/// in during this dispatch, so a cold store resolves without a
/// guest-visible entry. No file means no digest key, so no cache applies.
pub(super) fn dispatch(
    runner: &impl Runner, axis: Axis, name: &str, version: Option<&semver::Version>,
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
pub(super) async fn load<B: BlobStore>(
    runner: &impl Runner, blobs: &B, location: &AdapterLocation, axis: Axis, name: &str,
    version: Option<&semver::Version>,
) -> Result<Metadata, Error> {
    let (container, component) = slot(location);
    // An unreadable component digests as empty, matching the pre-seam
    // file reader; the cache then simply never hits.
    let bytes = blobs.get(container, &component).await.ok().flatten().unwrap_or_default();
    let digest = emery_diagnostics::cache::content_digest(&bytes);
    let cache_object = format!("{component}.metadata.json");
    if let Some(answer) = read_cache(blobs, container, &cache_object, &digest).await {
        return Ok(answer);
    }

    let adapter_id = RoutedId::new(axis, name, version.cloned()).to_string();
    let answer = runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })?;
    write_cache(blobs, container, &cache_object, &digest, &answer).await;
    Ok(answer)
}

async fn read_cache<B: BlobStore>(
    blobs: &B, container: &str, object: &str, digest: &str,
) -> Option<Metadata> {
    let raw = blobs.get(container, object).await.ok().flatten()?;
    let cache: MetadataCache = serde_json::from_slice(&raw).ok()?;
    (cache.digest == digest).then_some(cache.metadata)
}

// Best-effort, like the pre-seam sidecar writer: a failed cache write
// (e.g. the read-only store) never fails the resolve.
async fn write_cache<B: BlobStore>(
    blobs: &B, container: &str, object: &str, digest: &str, answer: &Metadata,
) {
    let cache = MetadataCache {
        digest: digest.to_string(),
        metadata: answer.clone(),
    };
    if let Ok(body) = serde_json::to_string_pretty(&cache) {
        drop(blobs.put(container, object, body.as_bytes()).await);
    }
}
