//! On-disk `meta.yaml` sidecar — schema, atomic read, atomic write.

use std::path::Path;
use std::{fs, io};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tempfile::Builder;

use super::SIDECAR_FILENAME;
use crate::error::ExtensionError;
use crate::manifest::{ExtensionPermissions, looks_like_sha256_hex};
use crate::package::PackageMetadata;

/// Currently supported sidecar schema version. Bumped on any breaking
/// shape change; readers must reject anything else with a schema
/// diagnostic.
pub(crate) const SIDECAR_SCHEMA_VERSION: u32 = 1;

/// On-disk `meta.yaml` metadata beside cached tool bytes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Sidecar {
    /// Sidecar schema version. Version `1` is the only supported shape.
    pub schema_version: u32,
    /// Scope segment, for example `project--my-app`.
    pub scope: String,
    /// Extension name from the declaration.
    pub tool_name: String,
    /// Extension version from the declaration.
    pub tool_version: String,
    /// Literal source string from the declaration.
    pub source: String,
    /// UTC timestamp from when the bytes were fetched or copied.
    #[serde(with = "specify_error::serde_rfc3339")]
    pub fetched_at: Timestamp,
    /// Fetch-time permissions snapshot. Informational only.
    pub permissions_snapshot: ExtensionPermissions,
    /// Optional lower-case hex SHA-256 digest copied from the declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Optional package metadata for wasm-pkg sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<PackageMetadata>,
}

/// Read `meta.yaml` from `path`.
///
/// Missing sidecars are returned as `Ok(None)` so callers can distinguish a
/// cold cache from a corrupt one and treat the former as a [`super::Status::MissNotFound`].
///
/// # Errors
///
/// Returns the `tool-io` diagnostic when the file exists but cannot be
/// read, the `tool-sidecar-parse` diagnostic when the bytes are not valid
/// YAML or do not deserialize into the v1 shape, and the
/// `tool-sidecar-schema` diagnostic when the parsed document violates a
/// schema invariant (`schema-version != 1`, an empty required field, or a
/// malformed `sha256` digest).
pub fn read_sidecar(path: &Path) -> Result<Option<Sidecar>, ExtensionError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(ExtensionError::cache_io("read sidecar", path, err)),
    };
    let sidecar: Sidecar = serde_saphyr::from_str(&contents)
        .map_err(|err| ExtensionError::sidecar_parse(path, err))?;
    validate_sidecar_schema(path, &sidecar)?;
    Ok(Some(sidecar))
}

/// Write `meta.yaml` to `path` via a sibling temporary file and atomic rename.
///
/// # Errors
///
/// Returns the `tool-sidecar-schema` diagnostic when `sidecar` does not
/// satisfy the v1 schema, the `tool-cache-root` diagnostic when `path`
/// has no parent directory, the `tool-io` diagnostic when the parent
/// directory cannot be created, a unique temp path cannot be allocated,
/// or the temp file cannot be written, and the `tool-atomic-move-failed`
/// diagnostic when the final rename into place fails (a crash here
/// leaves the destination untouched).
pub fn write_sidecar(path: &Path, sidecar: &Sidecar) -> Result<(), ExtensionError> {
    validate_sidecar_schema(path, sidecar)?;
    let Some(parent) = path.parent() else {
        return Err(ExtensionError::cache_root(format!(
            "sidecar path has no parent: {}",
            path.display()
        )));
    };
    fs::create_dir_all(parent)
        .map_err(|err| ExtensionError::cache_io("create sidecar parent", parent, err))?;
    let contents = serde_saphyr::to_string(sidecar).map_err(|err| {
        ExtensionError::sidecar_schema(path, format!("failed to serialize sidecar: {err}"))
    })?;
    let prefix = format!(".{SIDECAR_FILENAME}.");
    let tmp = Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|err| ExtensionError::cache_io("create sidecar temp", parent, err))?;
    fs::write(tmp.path(), contents)
        .map_err(|err| ExtensionError::cache_io("write sidecar temp", tmp.path(), err))?;
    tmp.persist(path).map(|_| ()).map_err(|err| {
        ExtensionError::atomic_move_failed(
            err.file.path().to_path_buf(),
            path.to_path_buf(),
            err.error,
        )
    })
}

fn validate_sidecar_schema(path: &Path, sidecar: &Sidecar) -> Result<(), ExtensionError> {
    if sidecar.schema_version != SIDECAR_SCHEMA_VERSION {
        return sidecar_schema_error(path, "schema-version must be 1");
    }
    for (field, value) in [
        ("scope", sidecar.scope.as_str()),
        ("tool-name", sidecar.tool_name.as_str()),
        ("tool-version", sidecar.tool_version.as_str()),
        ("source", sidecar.source.as_str()),
    ] {
        if value.is_empty() {
            return sidecar_schema_error(path, format!("{field} must not be empty"));
        }
    }
    if let Some(sha256) = &sidecar.sha256
        && !looks_like_sha256_hex(sha256)
    {
        return sidecar_schema_error(path, "sha256 must be 64 lowercase hexadecimal characters");
    }
    if let Some(package) = &sidecar.package {
        for (field, value) in [
            ("package.name", package.name.as_str()),
            ("package.version", package.version.as_str()),
            ("package.registry", package.registry.as_str()),
        ] {
            if value.is_empty() {
                return sidecar_schema_error(path, format!("{field} must not be empty"));
            }
        }
    }
    Ok(())
}

fn sidecar_schema_error(path: &Path, detail: impl Into<String>) -> Result<(), ExtensionError> {
    Err(ExtensionError::sidecar_schema(path, detail))
}
