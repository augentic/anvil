//! Adapter metadata values and component-sidecar caching.
//!
//! Dispatch runs through an explicitly supplied [`Runner`], keeping
//! deployment binding on the provider rather than in process-global
//! state. Component answers are cached against the component SHA-256.

use std::path::{Path, PathBuf};

use error::Error;
use serde::{Deserialize, Serialize};

use super::core::{AdapterLocation, Axis, BuildInputDeclaration, PlatformsCapability};

/// Unified metadata answer across both adapter axes.
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

/// One metadata dispatch by axis and routed adapter id.
#[derive(Debug)]
pub struct Request<'a> {
    /// The axis interface to invoke `metadata` on.
    pub axis: Axis,
    /// Routed adapter id (`<axis>:<name>`).
    pub adapter_id: &'a str,
}

/// Deployment-supplied metadata dispatcher.
pub type Runner = fn(&Request<'_>) -> Result<Metadata, Error>;

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

/// Load component metadata through `runner`, honoring the digest cache.
pub(super) fn load(
    runner: Runner, location: &AdapterLocation, axis: Axis, name: &str,
) -> Result<Metadata, Error> {
    let component = location.path();
    let digest = schema::cache::file_content_digest(component);
    let cache_path = metadata_cache_path(component);
    if let Some(answer) = read_cache(&cache_path, &digest) {
        return Ok(answer);
    }

    let adapter_id = format!("{}:{name}", axis_prefix(axis));
    let answer = runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })?;
    write_cache(&cache_path, &digest, &answer);
    Ok(answer)
}

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

fn write_cache(cache_path: &Path, digest: &str, answer: &Metadata) {
    let cache = MetadataCache {
        digest: digest.to_string(),
        metadata: answer.clone(),
    };
    if let Ok(body) = serde_json::to_string_pretty(&cache) {
        drop(std::fs::write(cache_path, body));
    }
}
