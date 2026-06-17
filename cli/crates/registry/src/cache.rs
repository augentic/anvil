//! Global cache layout and metadata helpers for resolved WASI tools.
//! Owns pure path/scope helpers and the [`Status`] decision API;
//! I/O concerns live in [`fetch`], [`gc`], and [`meta`].

use std::path::{Component, Path, PathBuf};
use std::{env, fs, io};

use crate::error::ExtensionError;
use crate::manifest::ExtensionScope;

pub mod fetch;
pub mod gc;
pub mod meta;

#[cfg(test)]
mod tests;

pub use fetch::stage_and_install;
pub use gc::scan as scan_for_gc;
pub(crate) use meta::SIDECAR_SCHEMA_VERSION;
pub use meta::{Sidecar, read_sidecar, write_sidecar};

/// Filename used for cached component bytes.
pub const MODULE_FILENAME: &str = "module.wasm";

/// Filename used for cached tool metadata.
pub const SIDECAR_FILENAME: &str = "meta.yaml";

/// Embedded JSON Schema for cache sidecars, re-exported from the
/// central [`specify_schema`] embed.
pub use specify_schema::EXTENSION_SIDECAR_JSON_SCHEMA;

/// Cache reuse state for a declared tool.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    /// Cached bytes and sidecar metadata match the live declaration tuple.
    Hit,
    /// The cache directory, module, or sidecar is absent.
    MissNotFound,
    /// Cached metadata exists but no longer matches the live declaration tuple.
    MissChanged,
}

/// Resolve the global tool cache root.
///
/// Precedence is `SPECIFY_EXTENSIONS_CACHE`, then `XDG_CACHE_HOME/specify/extensions`,
/// then `$HOME/.cache/specify/extensions`.
///
/// # Errors
///
/// Returns the `tool-cache-root` diagnostic when an explicit
/// `SPECIFY_EXTENSIONS_CACHE` or `XDG_CACHE_HOME` override is set but empty or
/// relative, when `HOME` is set but empty or relative, or when none of
/// the three precedence variables is set and so no fallback root can be
/// selected.
pub fn root() -> Result<PathBuf, ExtensionError> {
    if let Some(value) = env::var_os("SPECIFY_EXTENSIONS_CACHE") {
        return env_path("SPECIFY_EXTENSIONS_CACHE", value);
    }

    if let Some(value) = env::var_os("XDG_CACHE_HOME") {
        return env_path("XDG_CACHE_HOME", value).map(|root| root.join("specify").join("tools"));
    }

    if let Some(home) = env::var_os("HOME") {
        let home = env_path("HOME", home)?;
        return Ok(home.join(".cache").join("specify").join("tools"));
    }

    Err(ExtensionError::cache_root(
        "could not determine a cache directory from SPECIFY_EXTENSIONS_CACHE, XDG_CACHE_HOME, or HOME",
    ))
}

/// Convert a declaration scope into the on-disk scope segment.
///
/// # Errors
///
/// Returns the `tool-resolver` diagnostic when the project name or
/// plugin slug is empty, contains a path separator, equals `.` or `..`,
/// or contains a component that would escape the scope directory.
pub fn scope_segment(scope: &ExtensionScope) -> Result<String, ExtensionError> {
    match scope {
        ExtensionScope::Project { project_name } => {
            validate_segment("project name", project_name)?;
            Ok(format!("project--{project_name}"))
        }
        ExtensionScope::Plugin {
            axis, plugin_slug, ..
        } => {
            validate_segment("plugin slug", plugin_slug)?;
            Ok(format!("adapter--{axis}--{plugin_slug}"))
        }
    }
}

/// Compute the cache directory for one tool version.
///
/// # Errors
///
/// Returns the `tool-cache-root` diagnostic when no cache root can be
/// selected from the environment, and the `tool-resolver` diagnostic
/// when `name`, `version`, or the scope's project/plugin slug fails
/// segment validation.
pub fn tool_dir(
    scope: &ExtensionScope, name: &str, version: &str,
) -> Result<PathBuf, ExtensionError> {
    validate_segment("tool name", name)?;
    validate_segment("tool version", version)?;
    Ok(root()?.join(scope_segment(scope)?).join(name).join(version))
}

/// Compute the cached module path for one tool version.
///
/// # Errors
///
/// Forwards every error returned by [`tool_dir`].
pub fn module_path(
    scope: &ExtensionScope, name: &str, version: &str,
) -> Result<PathBuf, ExtensionError> {
    Ok(tool_dir(scope, name, version)?.join(MODULE_FILENAME))
}

/// Compute the cached sidecar path for one tool version.
///
/// # Errors
///
/// Forwards every error returned by [`tool_dir`].
pub fn sidecar_path(
    scope: &ExtensionScope, name: &str, version: &str,
) -> Result<PathBuf, ExtensionError> {
    Ok(tool_dir(scope, name, version)?.join(SIDECAR_FILENAME))
}

/// Return cache status for a live declaration tuple.
///
/// # Errors
///
/// Forwards every error returned by [`module_path()`], [`sidecar_path`], and
/// [`read_sidecar`] — the latter surfaces the `tool-io` diagnostic and
/// the `tool-sidecar-parse` / `tool-sidecar-schema` diagnostics when an
/// existing `meta.yaml` is unreadable or malformed (a missing sidecar is
/// reported as [`Status::MissNotFound`] rather than as an error).
pub fn status(
    scope: &ExtensionScope, tool_name: &str, tool_version: &str, source: &str, sha256: Option<&str>,
) -> Result<Status, ExtensionError> {
    let module = module_path(scope, tool_name, tool_version)?;
    if !module.is_file() {
        return Ok(Status::MissNotFound);
    }
    let Some(sidecar) = read_sidecar(&sidecar_path(scope, tool_name, tool_version)?)? else {
        return Ok(Status::MissNotFound);
    };
    sidecar_status(scope, tool_name, tool_version, source, sha256, &sidecar)
}

/// Compare an already-loaded sidecar against a live declaration tuple.
///
/// # Errors
///
/// Returns the `tool-resolver` diagnostic when the live scope cannot be
/// converted into a valid cache segment.
pub fn sidecar_status(
    scope: &ExtensionScope, tool_name: &str, tool_version: &str, source: &str,
    sha256: Option<&str>, sidecar: &Sidecar,
) -> Result<Status, ExtensionError> {
    let live_scope = scope_segment(scope)?;
    if sidecar.scope == live_scope
        && sidecar.tool_name == tool_name
        && sidecar.tool_version == tool_version
        && sidecar.source == source
        && sidecar.sha256.as_deref() == sha256
    {
        Ok(Status::Hit)
    } else {
        Ok(Status::MissChanged)
    }
}

fn env_path(name: &'static str, value: std::ffi::OsString) -> Result<PathBuf, ExtensionError> {
    if value.is_empty() {
        return Err(ExtensionError::cache_root(format!("{name} is set but empty")));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ExtensionError::cache_root(format!(
            "{name} must be an absolute path: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn validate_segment(field: &'static str, value: &str) -> Result<(), ExtensionError> {
    if value.is_empty() {
        return Err(invalid_segment(field, value, "must not be empty"));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(invalid_segment(field, value, "must not contain path separators"));
    }
    if value == "." || value == ".." {
        return Err(invalid_segment(field, value, "must not be a relative path segment"));
    }
    if Path::new(value).components().any(|component| {
        matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
    }) {
        return Err(invalid_segment(field, value, "must stay within the cache directory"));
    }
    Ok(())
}

fn invalid_segment(field: &'static str, value: &str, reason: &'static str) -> ExtensionError {
    ExtensionError::Diag {
        code: "tool-resolver",
        detail: format!("invalid tool cache segment `{value}` for {field}: {reason}"),
    }
}

fn sorted_dir_entries(path: &Path) -> Result<Vec<fs::DirEntry>, ExtensionError> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(path)
        .map_err(|err| ExtensionError::cache_io("read cache directory", path, err))?
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(|err| ExtensionError::cache_io("read cache directory entry", path, err))?;
    entries.sort_by_key(fs::DirEntry::path);
    Ok(entries)
}
