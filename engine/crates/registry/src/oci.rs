//! OCI artifact transport for adapter packages (RFC-48 D6, Shape B).
//!
//! An adapter publishes as a single-layer OCI artifact: the
//! byte-deterministic packed tree (`pack::pack_adapter`) is the one
//! layer, content-addressed by its sha256 digest. [`push_adapter`]
//! pushes the layer plus a minimal config under the immutable
//! `<registry>/<repo>:<version>` reference; [`pull_adapter`] fetches the
//! layer bytes back, and the caller verifies them against the recorded
//! immutable digest with [`crate::pack::verify_digest`] (the
//! verify-on-read gate, D4).
//!
//! The Step-1 spike established that `wkg` / `wasm-pkg-client` reject an
//! opaque (non-component) blob in both directions, so adapter transport
//! uses the raw OCI layer path here rather than the wasm-pkg package
//! path in `crate::package`.

use oci_client::client::{Config, ImageLayer};
pub use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

use crate::error::ExtensionError;
use crate::pack::ADAPTER_LAYER_MEDIA_TYPE;

/// Media type of the minimal adapter artifact config object.
const ADAPTER_CONFIG_MEDIA_TYPE: &str = "application/vnd.augentic.specify.adapter.config.v1+json";

/// Publish a packed adapter `layer` to the immutable `reference`
/// (`<registry>/<repo>:<version>`), returning the layer's content
/// digest (`sha256:<hex>`) — the value recorded for verify-on-read.
///
/// # Errors
///
/// Returns `adapter-transport-failed` when the reference is malformed or
/// the registry push fails.
pub fn push_adapter(
    reference: &str, layer: Vec<u8>, auth: &RegistryAuth,
) -> Result<String, ExtensionError> {
    let digest = crate::pack::content_digest(&layer);
    let image = parse_reference(reference)?;
    let runtime = build_runtime(reference)?;
    runtime.block_on(push_async(reference, &image, layer, auth))?;
    Ok(digest)
}

/// Pull the packed adapter layer bytes from `reference`.
///
/// The caller is responsible for the verify-on-read digest check
/// (`pack::verify_digest`) against the recorded immutable digest.
///
/// # Errors
///
/// Returns `adapter-transport-failed` when the reference is malformed,
/// the pull fails, or the artifact does not carry exactly one adapter
/// layer.
pub fn pull_adapter(reference: &str, auth: &RegistryAuth) -> Result<Vec<u8>, ExtensionError> {
    let image = parse_reference(reference)?;
    let runtime = build_runtime(reference)?;
    runtime.block_on(pull_async(reference, &image, auth))
}

/// Resolve registry credentials from the environment.
///
/// Mirrors the wasm-pkg credential path: a `SPECIFY_REGISTRY_TOKEN`
/// bearer token, or a `SPECIFY_REGISTRY_USERNAME` / `SPECIFY_REGISTRY_PASSWORD`
/// basic pair, else anonymous (public read).
#[must_use]
pub fn registry_auth_from_env() -> RegistryAuth {
    if let Some(token) = non_empty_env("SPECIFY_REGISTRY_TOKEN") {
        return RegistryAuth::Bearer(token);
    }
    match (non_empty_env("SPECIFY_REGISTRY_USERNAME"), non_empty_env("SPECIFY_REGISTRY_PASSWORD")) {
        (Some(user), Some(password)) => RegistryAuth::Basic(user, password),
        _ => RegistryAuth::Anonymous,
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

/// Canonical first-party adapter registry host. Must match the
/// publish-side `${SPECIFY_REGISTRY:-augentic.io}/<namespace>/<name>:<version>`
/// reference the `specify-adapters` release workflow pushes, and the
/// `specify -> augentic.io` namespace mapping `specify init` writes into a
/// consumer's `.specify/wasm-pkg.toml`.
const DEFAULT_ADAPTER_REGISTRY: &str = "augentic.io";

/// Derive the immutable OCI reference for a first-party adapter package
/// `<namespace>:<name>@<version>` as `<host>/<namespace>/<name>:<version>`.
///
/// `host` is `$SPECIFY_REGISTRY` when set and non-empty, else
/// `DEFAULT_ADAPTER_REGISTRY` — the same precedence the publish workflow
/// uses, so a `specify init specify:<name>@<ver>` install pulls back
/// exactly what `specify adapter publish` pushed.
#[must_use]
pub fn adapter_reference(namespace: &str, name: &str, version: &str) -> String {
    let host =
        non_empty_env("SPECIFY_REGISTRY").unwrap_or_else(|| DEFAULT_ADAPTER_REGISTRY.to_string());
    format!("{host}/{namespace}/{name}:{version}")
}

fn parse_reference(reference: &str) -> Result<Reference, ExtensionError> {
    reference.parse::<Reference>().map_err(|err| {
        ExtensionError::transport(reference, format!("invalid OCI reference: {err}"))
    })
}

fn build_runtime(reference: &str) -> Result<tokio::runtime::Runtime, ExtensionError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| ExtensionError::transport(reference, format!("create tokio runtime: {err}")))
}

async fn push_async(
    reference: &str, image: &Reference, layer: Vec<u8>, auth: &RegistryAuth,
) -> Result<(), ExtensionError> {
    let client = Client::default();
    let layers = vec![ImageLayer::new(layer, ADAPTER_LAYER_MEDIA_TYPE.to_string(), None)];
    let config = Config::new(b"{}".to_vec(), ADAPTER_CONFIG_MEDIA_TYPE.to_string(), None);
    client
        .push(image, &layers, config, auth, None)
        .await
        .map(|_response| ())
        .map_err(|err| ExtensionError::transport(reference, format!("push artifact: {err}")))
}

async fn pull_async(
    reference: &str, image: &Reference, auth: &RegistryAuth,
) -> Result<Vec<u8>, ExtensionError> {
    let client = Client::default();
    let data = client
        .pull(image, auth, vec![ADAPTER_LAYER_MEDIA_TYPE])
        .await
        .map_err(|err| ExtensionError::transport(reference, format!("pull artifact: {err}")))?;
    let mut layers = data.layers;
    match layers.len() {
        1 => Ok(layers.remove(0).data.to_vec()),
        other => Err(ExtensionError::transport(
            reference,
            format!("expected exactly one adapter layer, found {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // With no credential env vars set, the resolver falls back to
    // anonymous (public read) rather than panicking or inventing creds.
    #[test]
    fn auth_falls_back_to_anonymous() {
        use crate::test_support::EnvGuard;

        let _guard = crate::test_support::env_lock();
        let _token = EnvGuard::scoped("SPECIFY_REGISTRY_TOKEN", None);
        let _user = EnvGuard::scoped("SPECIFY_REGISTRY_USERNAME", None);
        let _password = EnvGuard::scoped("SPECIFY_REGISTRY_PASSWORD", None);
        assert!(matches!(registry_auth_from_env(), RegistryAuth::Anonymous));
    }

    #[test]
    fn reference_parse_rejects_garbage() {
        let err = parse_reference("not a reference!!").expect_err("garbage reference");
        assert!(matches!(
            err,
            ExtensionError::Diag {
                code: "adapter-transport-failed",
                ..
            }
        ));
    }

    // The derived install reference must equal the publish-side
    // `<host>/<namespace>/<name>:<version>` form, defaulting the host to
    // `augentic.io` and honouring a `SPECIFY_REGISTRY` override.
    #[test]
    fn adapter_reference_defaults_and_overrides_host() {
        use std::path::Path;

        use crate::test_support::EnvGuard;

        let _guard = crate::test_support::env_lock();
        let unset = EnvGuard::scoped("SPECIFY_REGISTRY", None);
        assert_eq!(
            adapter_reference("specify", "omnia", "1.2.0"),
            "augentic.io/specify/omnia:1.2.0"
        );
        drop(unset);

        let _set = EnvGuard::scoped("SPECIFY_REGISTRY", Some(Path::new("ghcr.io/augentic")));
        assert_eq!(
            adapter_reference("specify", "omnia", "1.2.0"),
            "ghcr.io/augentic/specify/omnia:1.2.0"
        );
    }
}
