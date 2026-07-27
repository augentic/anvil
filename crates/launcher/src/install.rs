//! Host-owned package install: pull-on-miss from the first-party OCI
//! registry into the immutable global adapter store.
//!
//! The only download path in the deployment. A pinned routed id whose
//! store entry is absent is pulled anonymously as a standard Wasm OCI
//! artifact (`ghcr.io/augentic/emery-adapters/<name>:<version>`),
//! validated (single layer, wasm magic, size, manifest layer digest),
//! written atomically, and recorded in the digest sidecar with its
//! OCI provenance (repository, manifest digest, layer digest). The
//! guest never downloads: package installation is native-only.

use error::Error;
use oci_client::Reference;
use oci_client::client::{Client, ClientConfig, ClientProtocol};
use oci_client::secrets::RegistryAuth;
use oci_wasm::WasmClient;
use project::handler::ExecutionPaths;

/// Fixed first-party OCI repository prefix. The `emery:` namespace
/// maps to exactly one registry, compiled in — no configuration
/// surface (project-local or otherwise) can redirect it.
const FIRST_PARTY_REPOSITORY: &str = "ghcr.io/augentic/emery-adapters";

/// Upper bound on an installable component layer, in bytes.
const MAX_COMPONENT_BYTES: usize = 256 * 1024 * 1024;

/// The WebAssembly binary magic every component layer must open with.
const WASM_MAGIC: &[u8] = b"\0asm";

/// The OCI registry base one launcher pulls first-party packages
/// from: a repository prefix plus the client protocol.
#[derive(Clone, Debug)]
pub struct Registry {
    prefix: String,
    protocol: ClientProtocol,
}

impl Registry {
    /// The shipped composition: the compiled first-party GHCR
    /// repository over HTTPS.
    pub(crate) fn first_party() -> Self {
        Self {
            prefix: FIRST_PARTY_REPOSITORY.to_string(),
            protocol: ClientProtocol::Https,
        }
    }

    /// A plain-HTTP registry base — the integration-test seam over an
    /// in-process OCI server (e.g. `localhost:<port>/adapters`).
    /// Production composition never constructs this: the shipped
    /// binary hard-codes the first-party GHCR base and exposes no
    /// configuration surface that reaches here.
    #[must_use]
    pub fn insecure(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            protocol: ClientProtocol::Http,
        }
    }
}

/// Pull one pinned package from the registry and install it into the
/// global store with its digest sidecar (verify-after-write).
///
/// # Errors
///
/// `adapter-install-failed` when the registry pull fails (network,
/// missing tag, auth); `adapter-install-invalid` when the served
/// artifact is malformed (layer count, empty/oversized layer, missing
/// wasm magic, digest disagreement); `adapter-digest-mismatch` when
/// the freshly written entry fails verify-after-write.
pub async fn install(
    registry: &Registry, name: &str, version: &str, paths: &ExecutionPaths,
) -> Result<(), Error> {
    let repository = format!("{}/{name}", registry.prefix);
    let reference: Reference =
        format!("{repository}:{version}").parse().map_err(|err| Error::Diag {
            code: "adapter-install-invalid",
            detail: format!(
                "`emery:{name}@{version}` does not form a valid OCI reference under \
                 {repository}: {err}"
            ),
        })?;

    let client = WasmClient::new(Client::new(ClientConfig {
        protocol: registry.protocol.clone(),
        ..ClientConfig::default()
    }));
    let image =
        client.pull(&reference, &RegistryAuth::Anonymous).await.map_err(|err| Error::Diag {
            code: "adapter-install-failed",
            detail: format!("failed to install `emery:{name}@{version}` from {reference}: {err:#}"),
        })?;

    let invalid = |detail: String| Error::Diag {
        code: "adapter-install-invalid",
        detail: format!("refusing `emery:{name}@{version}` from {reference}: {detail}"),
    };

    // `WasmClient::pull` already enforces one wasm layer and the wasm
    // config media type; re-assert the count so the invariant is local.
    if image.layers.len() != 1 {
        return Err(invalid(format!(
            "expected exactly one component layer, got {}",
            image.layers.len()
        )));
    }
    let bytes = &image.layers[0].data;
    if bytes.is_empty() {
        return Err(invalid("the component layer is empty".to_string()));
    }
    if bytes.len() > MAX_COMPONENT_BYTES {
        return Err(invalid(format!(
            "the component layer is {} bytes, above the {MAX_COMPONENT_BYTES}-byte cap",
            bytes.len()
        )));
    }
    if !bytes.starts_with(WASM_MAGIC) {
        return Err(invalid("the component layer is not WebAssembly".to_string()));
    }
    let manifest =
        image.manifest.as_ref().ok_or_else(|| invalid("no manifest returned".to_string()))?;
    let layer_digest = format!("sha256:{}", diagnostics::digest::sha256_hex(bytes));
    if manifest.layers[0].digest != layer_digest {
        return Err(invalid(format!(
            "layer digest disagreement: manifest declares {}, content is {layer_digest}",
            manifest.layers[0].digest
        )));
    }
    let manifest_digest = image
        .digest
        .clone()
        .ok_or_else(|| invalid("no resolved manifest digest returned".to_string()))?;

    let locations = paths.locations();
    std::fs::create_dir_all(locations.store_root())?;
    let entry = locations.store_entry(name, version);
    artifacts::atomic::bytes_write(&entry, bytes)?;

    let tree_digest = diagnostics::cache::file_content_digest(&entry);
    let provenance = diagnostics::cache::OciProvenance {
        repository,
        manifest_digest,
        layer_digest,
    };
    let meta = locations.store_meta(name, version);
    diagnostics::cache::write_store_meta(&meta, &tree_digest, Some(&provenance))?;
    diagnostics::cache::verify_store_entry(&entry, &meta).map_err(|failure| Error::Diag {
        code: "adapter-digest-mismatch",
        detail: format!("store entry {} failed verify-after-write: {failure:?}", entry.display()),
    })?;
    Ok(())
}
