//! `specify adapter *` dispatcher — self-contained adapter build and
//! immutable OCI publish (RFC-48 D1/D4/D6/D9/D12).
//!
//! `build` packs an adapter directory into a byte-deterministic layer,
//! dereferencing the in-repo `adapters/shared/` symlinks into real bytes
//! and excluding the Rust source trees so the artifact ships prose plus
//! the committed `guest.wasm`. `publish` packs, pushes the single-layer
//! OCI artifact, pulls it back, and verifies the recorded digest.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specify_error::{Error, Result};
use specify_registry::{oci, pack};
use specify_workflow::adapter::ADAPTER_FILENAME;

use crate::runtime::cli::Format;
use crate::runtime::output;

/// Path-component names of Rust source trees excluded from every
/// published artifact — adapters ship the compiled `guest.wasm`, never
/// the Rust source (RFC-48 D10, RFC-61).
const SOURCE_EXCLUDES: &[&str] = &["extension", "crates", "src", "target"];

/// Minimal projection of `adapter.yaml` for the build / publish flow —
/// just the identity. The full axis-specific manifest shape is
/// validated at resolve time, not here.
#[derive(Debug, Deserialize)]
struct BuildManifest {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct BuildBody {
    name: String,
    version: String,
    digest: String,
    layer_bytes: usize,
}

#[derive(Serialize)]
struct PublishBody {
    name: String,
    version: String,
    reference: String,
    digest: String,
    layer_bytes: usize,
}

/// `specify adapter build` — pack the adapter at `path` and report the
/// layer digest.
pub fn build(format: Format, path: &Path) -> Result<()> {
    let manifest = read_manifest(path)?;
    let layer = pack::pack_adapter(path, SOURCE_EXCLUDES)?;
    let body = BuildBody {
        name: manifest.name,
        version: manifest.version,
        digest: pack::content_digest(&layer),
        layer_bytes: layer.len(),
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_build_text)?;
    Ok(())
}

/// `specify adapter publish` — build, pack, push the single-layer OCI
/// artifact to `reference`, pull it back, and verify the recorded digest
/// (RFC-48 D4/D6). Refuses to overwrite an existing `(name, version)`
/// with different bytes.
pub fn publish(format: Format, path: &Path, reference: &str) -> Result<()> {
    let manifest = read_manifest(path)?;

    let layer = pack::pack_adapter(path, SOURCE_EXCLUDES)?;
    let layer_bytes = layer.len();
    let digest = pack::content_digest(&layer);
    let auth = oci::registry_auth_from_env();

    reject_republish_with_different_bytes(reference, &digest, &auth)?;

    oci::push_adapter(reference, layer, &auth)?;
    // Verify-on-read: pull the just-pushed artifact back and confirm the
    // bytes hash to the recorded digest before declaring success.
    let pulled = oci::pull_adapter(reference, &auth)?;
    pack::verify_digest(reference, &pulled, &digest)?;

    let body = PublishBody {
        name: manifest.name,
        version: manifest.version,
        reference: reference.to_string(),
        digest,
        layer_bytes,
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_publish_text)?;
    Ok(())
}

/// Refuse to overwrite an immutable `(name, version)` with different
/// bytes. A pull miss (artifact absent, or registry unreachable) means
/// nothing is published yet, so the push proceeds.
fn reject_republish_with_different_bytes(
    reference: &str, digest: &str, auth: &oci::RegistryAuth,
) -> Result<()> {
    oci::pull_adapter(reference, auth).map_or(Ok(()), |existing| {
        let existing_digest = pack::content_digest(&existing);
        if existing_digest == digest {
            Ok(())
        } else {
            Err(Error::Diag {
                code: "adapter-republish-conflict",
                detail: format!(
                    "{reference} already published with digest {existing_digest}; refusing to \
                     overwrite with {digest}"
                ),
            })
        }
    })
}

fn read_manifest(path: &Path) -> Result<BuildManifest> {
    let manifest_path = path.join(ADAPTER_FILENAME);
    let raw = std::fs::read_to_string(&manifest_path).map_err(|err| Error::Diag {
        code: "adapter-build-failed",
        detail: format!("read {}: {err}", manifest_path.display()),
    })?;
    serde_saphyr::from_str(&raw).map_err(|err| Error::Diag {
        code: "adapter-build-failed",
        detail: format!("parse {}: {err}", manifest_path.display()),
    })
}

fn write_build_text(w: &mut dyn std::io::Write, body: &BuildBody) -> std::io::Result<()> {
    writeln!(w, "{}@{}", body.name, body.version)?;
    writeln!(w, "  digest: {}", body.digest)?;
    writeln!(w, "  layer-bytes: {}", body.layer_bytes)
}

fn write_publish_text(w: &mut dyn std::io::Write, body: &PublishBody) -> std::io::Result<()> {
    writeln!(w, "published {}@{}", body.name, body.version)?;
    writeln!(w, "  reference: {}", body.reference)?;
    writeln!(w, "  digest: {}", body.digest)?;
    writeln!(w, "  layer-bytes: {}", body.layer_bytes)
}
