//! Host-owned package install: pull-on-miss from the first-party OCI
//! registry into the immutable global adapter store.
//!
//! The only download path in the deployment. A pinned routed id whose
//! store entry is absent is pulled anonymously as a standard Wasm OCI
//! artifact (`ghcr.io/augentic/emery-adapters/<name>:<version>`),
//! validated (single layer, wasm magic, size, manifest layer digest),
//! and written atomically — the digest sidecar (with its OCI
//! provenance: repository, manifest digest, layer digest) first, the
//! component last, so a torn install never leaves an unverifiable
//! component behind. An
//! unpinned name with nothing local — or one the operator explicitly
//! updates — resolves its version first through [`resolve_latest`]
//! (the repository's newest exact-SemVer tag) and installs through
//! the same leg. The guest never downloads: package installation is
//! native-only.

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

/// One anonymous OCI client over the registry's protocol.
fn client(registry: &Registry) -> Client {
    Client::new(ClientConfig {
        protocol: registry.protocol.clone(),
        ..ClientConfig::default()
    })
}

/// Page size for tag listing.
const TAGS_PAGE: usize = 100;

/// Resolve the newest published version of `name`: list the
/// repository's tags, keep the exact-SemVer ones, take the maximum.
///
/// The explicit-update and pull-on-miss provisioning legs share this
/// kernel; normal resolution never reaches it while a local component
/// (cache seed or store entry) exists.
///
/// # Errors
///
/// `adapter-latest-failed` when the registry cannot be reached or the
/// listing fails (offline, unknown repository, auth);
/// `adapter-latest-none` when the repository serves no exact-SemVer
/// tag.
pub async fn resolve_latest(registry: &Registry, name: &str) -> Result<semver::Version, Error> {
    let repository = format!("{}/{name}", registry.prefix);
    let reference: Reference = repository.parse().map_err(|err| Error::Diag {
        code: "adapter-latest-failed",
        detail: format!("`{name}` does not form a valid OCI repository at {repository}: {err}"),
    })?;

    let client = client(registry);
    let mut tags: Vec<String> = Vec::new();
    let mut last: Option<String> = None;
    loop {
        let page = client
            .list_tags(&reference, &RegistryAuth::Anonymous, Some(TAGS_PAGE), last.as_deref())
            .await
            .map_err(|err| Error::Diag {
                code: "adapter-latest-failed",
                detail: format!(
                    "failed to list published versions of `{name}` at {repository}: {err:#}. \
                     Check the adapter name is spelled correctly and the registry is reachable; \
                     seed a local component instead with `emery adapter add \
                     <path/to/{name}.wasm>`, or pin a published version (`emery:{name}@<semver>`)"
                ),
            })?;
        let batch = page.tags.len();
        // Guard against servers that ignore pagination and re-serve
        // the same page.
        if batch == 0 || page.tags.last() == last.as_ref() {
            break;
        }
        last = page.tags.last().cloned();
        tags.extend(page.tags);
        if batch < TAGS_PAGE {
            break;
        }
    }

    tags.iter().filter_map(|tag| semver::Version::parse(tag).ok()).max().ok_or_else(|| {
        Error::Diag {
            code: "adapter-latest-none",
            detail: format!(
                "no published exact-SemVer version of `{name}` found at {repository}; seed a \
                 local component with `emery adapter add <path/to/{name}.wasm>` or pin a \
                 published version (`emery:{name}@<semver>`)"
            ),
        }
    })
}

/// The newest version of `name` already installed in the global
/// store, from the `<name>@<version>.wasm` entry filenames. `None`
/// when nothing local exists (or the store root is absent).
pub fn store_newest(name: &str, paths: &ExecutionPaths) -> Option<semver::Version> {
    let prefix = format!("{name}@");
    let entries = std::fs::read_dir(paths.locations().store_root()).ok()?;
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|file| {
            file.strip_prefix(&prefix)
                .and_then(|rest| rest.strip_suffix(".wasm"))
                .and_then(|version| semver::Version::parse(version).ok())
        })
        .max()
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

    let client = WasmClient::new(client(registry));
    let image =
        client.pull(&reference, &RegistryAuth::Anonymous).await.map_err(|err| Error::Diag {
            code: "adapter-install-failed",
            detail: format!(
                "failed to install `emery:{name}@{version}` from {reference}: {err:#}. Check the \
                 adapter name is spelled correctly and the version is published; seed a local \
                 component instead with `emery adapter add <path/to/{name}.wasm>`, or pin a \
                 different published version (`emery:{name}@<semver>`)"
            ),
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
    if manifest.layers.len() != 1 {
        return Err(invalid(format!(
            "manifest declares {} layers, expected exactly one",
            manifest.layers.len()
        )));
    }
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

    // Sidecar first, component last: resolution keys "installed" on
    // the component file, so a tear between the two writes leaves at
    // most an orphan sidecar the next install overwrites — never an
    // unverifiable component. The entry's content digest is the layer
    // digest (the store entry is the layer bytes).
    let locations = paths.locations();
    std::fs::create_dir_all(locations.store_root())?;
    let entry = locations.store_entry(name, version);
    let meta = locations.store_meta(name, version);
    let provenance = diagnostics::cache::OciProvenance {
        repository,
        manifest_digest,
        layer_digest: layer_digest.clone(),
    };
    diagnostics::cache::write_store_meta(&meta, &layer_digest, Some(&provenance))?;
    artifacts::atomic::bytes_write(&entry, bytes)?;
    diagnostics::cache::verify_store_entry(&entry, &meta).map_err(|failure| Error::Diag {
        code: "adapter-digest-mismatch",
        detail: format!("store entry {} failed verify-after-write: {failure:?}", entry.display()),
    })?;
    Ok(())
}
