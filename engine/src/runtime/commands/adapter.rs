//! `specify adapter *` dispatcher — self-contained adapter build and
//! immutable OCI publish (RFC-48 D1/D4/D6/D9/D10/D12).
//!
//! `build` packs an adapter directory into a byte-deterministic layer,
//! dereferencing the in-repo `adapters/shared/` symlinks into real bytes
//! and excluding the declared `extension/` source so the artifact is
//! self-contained. `publish` packs, pushes the single-layer OCI
//! artifact, pulls it back, and verifies the recorded digest.

pub mod cli;

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use specify_error::{Error, Result};
use specify_registry::{oci, pack};
use specify_workflow::adapter::{ADAPTER_FILENAME, ADAPTER_WASM_FILENAME};

use crate::runtime::cli::Format;
use crate::runtime::output;

/// Path-component name of the co-located extension source crate. Excluded
/// from every published artifact — adapters ship the compiled
/// `adapter.wasm`, never the Rust source (RFC-48 D10).
const EXTENSION_SOURCE_DIR: &str = "extension";

/// `wasm32-wasip2` is the component target every first-party extension
/// compiles to (Omnia SDK constraint).
const WASM_TARGET: &str = "wasm32-wasip2";

/// Minimal projection of `adapter.yaml` for the build / publish flow —
/// just the identity and whether an extension is declared. The full
/// axis-specific manifest shape is validated at resolve time, not here.
#[derive(Debug, Deserialize)]
struct BuildManifest {
    name: String,
    version: String,
    #[serde(default)]
    extension: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct BuildBody {
    name: String,
    version: String,
    digest: String,
    layer_bytes: usize,
    extension_declared: bool,
    wasm_built: bool,
    dry_run: bool,
}

#[derive(Serialize)]
struct PublishBody {
    name: String,
    version: String,
    reference: String,
    digest: String,
    layer_bytes: usize,
}

/// `specify adapter build` — pack the adapter at `path` (optionally
/// compiling its extension to the committed `adapter.wasm` first) and
/// report the layer digest.
pub fn build(format: Format, path: &Path, dry_run: bool, refresh_extension: bool) -> Result<()> {
    let manifest = read_manifest(path)?;
    let extension_declared = manifest.extension.is_some();
    let wasm_built = if dry_run {
        false
    } else {
        ensure_extension_wasm(path, extension_declared, refresh_extension)?
    };

    let layer = pack::pack_adapter(path, &[EXTENSION_SOURCE_DIR])?;
    let body = BuildBody {
        name: manifest.name,
        version: manifest.version,
        digest: pack::content_digest(&layer),
        layer_bytes: layer.len(),
        extension_declared,
        wasm_built,
        dry_run,
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
    ensure_extension_wasm(path, manifest.extension.is_some(), false)?;

    let layer = pack::pack_adapter(path, &[EXTENSION_SOURCE_DIR])?;
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

/// Ensure the committed `adapter.wasm` exists when an extension is
/// declared, compiling the co-located `extension/` crate to
/// `wasm32-wasip2` when the wasm is absent or `refresh` is set. Returns
/// whether a compile ran. Prose-only adapters and adapters whose wasm is
/// already committed never invoke cargo (RFC-48 D10).
fn ensure_extension_wasm(path: &Path, extension_declared: bool, refresh: bool) -> Result<bool> {
    if !extension_declared {
        return Ok(false);
    }
    let committed = path.join(ADAPTER_WASM_FILENAME);
    if committed.is_file() && !refresh {
        return Ok(false);
    }
    let crate_dir = path.join(EXTENSION_SOURCE_DIR);
    if !crate_dir.is_dir() {
        return Err(Error::Diag {
            code: "adapter-build-failed",
            detail: format!(
                "extension declared but no `{EXTENSION_SOURCE_DIR}/` crate at {}",
                crate_dir.display()
            ),
        });
    }
    compile_extension(&crate_dir, &committed)?;
    Ok(true)
}

fn compile_extension(crate_dir: &Path, committed: &Path) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(crate_dir)
        .args(["build", "--release", "--target", WASM_TARGET])
        .status()
        .map_err(|err| Error::Diag {
            code: "adapter-build-failed",
            detail: format!("spawn cargo for {}: {err}", crate_dir.display()),
        })?;
    if !status.success() {
        return Err(Error::Diag {
            code: "adapter-build-failed",
            detail: format!("cargo build failed for {}", crate_dir.display()),
        });
    }
    let produced = sole_wasm_artifact(crate_dir)?;
    std::fs::copy(&produced, committed).map_err(|err| Error::Diag {
        code: "adapter-build-failed",
        detail: format!("copy {} -> {}: {err}", produced.display(), committed.display()),
    })?;
    Ok(())
}

/// Resolve the single `*.wasm` cargo produced under
/// `<crate>/target/wasm32-wasip2/release/`. Exactly one is expected; a
/// zero / multiple count is an `adapter-build-failed`.
fn sole_wasm_artifact(crate_dir: &Path) -> Result<PathBuf> {
    let release = crate_dir.join("target").join(WASM_TARGET).join("release");
    let read = std::fs::read_dir(&release).map_err(|err| Error::Diag {
        code: "adapter-build-failed",
        detail: format!("read {}: {err}", release.display()),
    })?;
    let mut wasm: Vec<PathBuf> = read
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    match wasm.len() {
        1 => Ok(wasm.remove(0)),
        other => Err(Error::Diag {
            code: "adapter-build-failed",
            detail: format!("expected exactly one wasm under {}, found {other}", release.display()),
        }),
    }
}

fn write_build_text(w: &mut dyn std::io::Write, body: &BuildBody) -> std::io::Result<()> {
    writeln!(w, "{}@{}", body.name, body.version)?;
    writeln!(w, "  digest: {}", body.digest)?;
    writeln!(w, "  layer-bytes: {}", body.layer_bytes)?;
    writeln!(w, "  extension-declared: {}", body.extension_declared)?;
    writeln!(w, "  wasm-built: {}", body.wasm_built)?;
    writeln!(w, "  dry-run: {}", body.dry_run)
}

fn write_publish_text(w: &mut dyn std::io::Write, body: &PublishBody) -> std::io::Result<()> {
    writeln!(w, "published {}@{}", body.name, body.version)?;
    writeln!(w, "  reference: {}", body.reference)?;
    writeln!(w, "  digest: {}", body.digest)?;
    writeln!(w, "  layer-bytes: {}", body.layer_bytes)
}
