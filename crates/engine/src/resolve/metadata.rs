//! Adapter metadata dispatch and digest-keyed caching.

use emery_error::Error;
use omnia_guest::BlobStore;
use serde::{Deserialize, Serialize};

use super::core::Axis;
use super::routed::RoutedId;
use crate::handler::ADAPTERS_CONTAINER;

/// A source adapter's metadata answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Metadata {
    /// Optional Emery CLI compatibility floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emery_floor: Option<String>,
}

/// Metadata dispatch request.
#[derive(Debug)]
pub struct Request<'a> {
    /// Adapter axis.
    pub axis: Axis,
    /// Exact routed adapter id.
    pub adapter_id: &'a str,
}

/// Deployment-supplied metadata dispatch.
pub trait Runner: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync {}

impl<F: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync> Runner for F {}

/// Creates metadata dispatch over the provider's `Source` capability.
///
/// Target requests return `adapter-axis-removed`.
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

/// Dispatches metadata without guest-visible component access or caching.
///
/// The host may fault in a cold component during dispatch, before a
/// guest-visible file exists to key a cache.
pub(super) fn dispatch(
    runner: &impl Runner, axis: Axis, name: &str, version: Option<&semver::Version>,
) -> Result<Metadata, Error> {
    let adapter_id = RoutedId::new(axis, name, version.cloned()).to_string();
    runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })
}

/// Loads component metadata through a digest-keyed sidecar cache.
pub(super) async fn load<B: BlobStore>(
    runner: &impl Runner, blobs: &B, component: &str, bytes: &[u8], axis: Axis, name: &str,
) -> Result<Metadata, Error> {
    let digest = emery_diagnostics::cache::content_digest(bytes);
    let cache_object = format!("{component}.metadata.json");
    if let Some(answer) = read_cache(blobs, ADAPTERS_CONTAINER, &cache_object, &digest).await {
        return Ok(answer);
    }

    let adapter_id = RoutedId::new(axis, name, None).to_string();
    let answer = runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })?;
    write_cache(blobs, ADAPTERS_CONTAINER, &cache_object, &digest, &answer).await;
    Ok(answer)
}

async fn read_cache<B: BlobStore>(
    blobs: &B, container: &str, object: &str, digest: &str,
) -> Option<Metadata> {
    let raw = blobs.get(container, object).await.ok().flatten()?;
    let cache: MetadataCache = serde_json::from_slice(&raw).ok()?;
    (cache.digest == digest).then_some(cache.metadata)
}

// Cache writes are advisory and never fail resolution.
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
