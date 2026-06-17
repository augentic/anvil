//! Source resolution for local paths, `file:` URIs, `https:` URIs,
//! `$PROJECT_DIR`/`$CAPABILITY_DIR` template paths, and wasm-pkg
//! package requests.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{Builder, NamedTempFile};

pub mod digest;
pub mod http;
pub mod local;

use specify_schema::digest::sha256_hex;

use crate::cache::{self, MODULE_FILENAME, SIDECAR_FILENAME, SIDECAR_SCHEMA_VERSION, Sidecar};
use crate::error::ExtensionError;
use crate::manifest::{Extension, ExtensionScope, ExtensionSource, PackageRequest};
use crate::package::{self, AcquiredBytes, persist_temp};

/// Cached component bytes plus the live declaration data needed to run them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExtension {
    /// Path to the cached WASI component bytes.
    pub bytes_path: PathBuf,
    /// Declaration site that supplied the tool.
    pub scope: ExtensionScope,
    /// Live manifest declaration used for argv and permission evaluation.
    pub tool: Extension,
}

/// Resolve a declared tool source into the global immutable cache.
///
/// A cache hit is valid only when the live declaration tuple matches the
/// sidecar and, when `sha256` is pinned, the cached bytes still hash to that
/// digest. Misses and digest refreshes stage `module.wasm` and `meta.yaml`
/// together, then atomically install the complete version directory.
///
/// `now` records the sidecar `fetched_at`; the dispatcher passes
/// `Timestamp::now`, tests pin a deterministic stamp.
///
/// `project_dir` is used to discover the project-local
/// `.specify/wasm-pkg.toml` (when present) so package fetches inherit
/// the project's namespace overrides without an env var.
///
/// # Errors
///
/// Returns cache errors, source read errors, digest mismatches, or typed network
/// resolver errors.
pub fn resolve(
    scope: &ExtensionScope, tool: &Extension, now: jiff::Timestamp, project_dir: &Path,
) -> Result<ResolvedExtension, ExtensionError> {
    resolve_with(scope, tool, now, project_dir, |request, dest_hint| {
        package::fetch(project_dir, request, dest_hint)
    })
}

/// Resolve a declared tool using an injected package fetcher.
///
/// Tests use this to cover package resolver behavior without depending on a
/// live registry.
///
/// # Errors
///
/// Returns the same cache, source, digest, and resolver errors as [`resolve`].
pub(crate) fn resolve_with(
    scope: &ExtensionScope, tool: &Extension, now: jiff::Timestamp, project_dir: &Path,
    package_fetch: impl Fn(&PackageRequest, &Path) -> Result<AcquiredBytes, ExtensionError>,
) -> Result<ResolvedExtension, ExtensionError> {
    let source = tool.source.to_wire_string().into_owned();
    let module = cache::module_path(scope, &tool.name, &tool.version)?;
    if cache::status(scope, &tool.name, &tool.version, &source, tool.sha256.as_deref())?
        == cache::Status::Hit
        && digest::cached_matches(&module, tool.sha256.as_deref())?
    {
        return Ok(resolved(scope, tool, module));
    }

    let dest = cache::tool_dir(scope, &tool.name, &tool.version)?;
    let Some(parent) = dest.parent() else {
        return Err(ExtensionError::cache_root(format!(
            "tool cache destination has no parent: {}",
            dest.display()
        )));
    };
    fs::create_dir_all(parent)
        .map_err(|err| ExtensionError::cache_io("create resolver staging parent", parent, err))?;
    let stem = dest.file_name().unwrap_or_else(|| OsStr::new("tool")).to_string_lossy();
    let prefix = format!(".resolver-{stem}.");
    let staged = Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .tempdir_in(parent)
        .map_err(|err| ExtensionError::cache_io("create resolver staging directory", parent, err))?
        .keep();
    let capability_dir = match scope {
        ExtensionScope::Plugin { capability_dir, .. } => Some(capability_dir.as_path()),
        ExtensionScope::Project { .. } => None,
    };
    let expanded_source =
        tool.source.expand(project_dir, capability_dir).map_err(|reason| ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!("tool source `{}` is invalid: {reason}", tool.source.to_wire_string()),
        })?;
    let install_result = stage_and_install(
        &StageRequest {
            scope,
            tool,
            source: &source,
            staged: &staged,
            dest: &dest,
            now,
            expanded_source: &expanded_source,
        },
        package_fetch,
    );
    // The atomic install moves `staged/` into `dest/`, so its absence on
    // success is expected. On failure we tear down the staging tree.
    let cleanup_result = if install_result.is_ok() && !staged.exists() {
        Ok(())
    } else {
        fs::remove_dir_all(&staged)
    };
    match (install_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(resolved(scope, tool, dest.join(MODULE_FILENAME))),
        (Ok(()), Err(err)) => {
            Err(ExtensionError::cache_io("remove resolver staging directory", staged, err))
        }
        (Err(err), _) => Err(err),
    }
}

/// Borrowed inputs for [`stage_and_install`], grouping the staging
/// pipeline's resolution-time facts so the function stays a two-argument
/// (request + fetch closure) boundary.
struct StageRequest<'a> {
    scope: &'a ExtensionScope,
    tool: &'a Extension,
    source: &'a str,
    staged: &'a Path,
    dest: &'a Path,
    now: jiff::Timestamp,
    expanded_source: &'a ExtensionSource,
}

fn stage_and_install(
    req: &StageRequest<'_>,
    package_fetch: impl Fn(&PackageRequest, &Path) -> Result<AcquiredBytes, ExtensionError>,
) -> Result<(), ExtensionError> {
    let module_dest = req.staged.join(MODULE_FILENAME);
    let acquired = acquire_source_bytes(req.expanded_source, &module_dest, package_fetch)?;
    digest::validate(req.source, &acquired, req.tool.sha256.as_deref())?;
    let AcquiredBytes {
        temp,
        package_metadata,
        ..
    } = acquired;
    persist_temp(temp, &module_dest)?;
    let sidecar = Sidecar {
        schema_version: SIDECAR_SCHEMA_VERSION,
        scope: cache::scope_segment(req.scope)?,
        tool_name: req.tool.name.clone(),
        tool_version: req.tool.version.clone(),
        source: req.source.to_string(),
        fetched_at: req.now,
        permissions_snapshot: req.tool.permissions.clone(),
        sha256: req.tool.sha256.clone(),
        package: package_metadata,
    };
    cache::write_sidecar(&req.staged.join(SIDECAR_FILENAME), &sidecar)?;
    cache::stage_and_install(req.staged, req.dest)
}

fn resolved(scope: &ExtensionScope, tool: &Extension, bytes_path: PathBuf) -> ResolvedExtension {
    ResolvedExtension {
        bytes_path,
        scope: scope.clone(),
        tool: tool.clone(),
    }
}

fn acquire_source_bytes(
    source: &ExtensionSource, dest_hint: &Path,
    package_fetch: impl Fn(&PackageRequest, &Path) -> Result<AcquiredBytes, ExtensionError>,
) -> Result<AcquiredBytes, ExtensionError> {
    match source {
        ExtensionSource::LocalPath(path) => buffered_into_acquired(
            &local::read_local_path(path, &path.to_string_lossy())?,
            dest_hint,
        ),
        ExtensionSource::FileUri(uri) => {
            buffered_into_acquired(&local::read_file_uri(uri)?, dest_hint)
        }
        ExtensionSource::HttpsUri(url) => http::download_https(url, dest_hint),
        ExtensionSource::Package(package) => package_fetch(package, dest_hint),
        ExtensionSource::TemplatePath(t) => Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{t}` is invalid: template source was not expanded before resolution"
            ),
        }),
    }
}

fn buffered_into_acquired(bytes: &[u8], dest_hint: &Path) -> Result<AcquiredBytes, ExtensionError> {
    let parent = dest_hint.parent().ok_or_else(|| {
        ExtensionError::cache_root(format!(
            "tool staging destination has no parent: {}",
            dest_hint.display()
        ))
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| ExtensionError::cache_io("create local staging parent", parent, err))?;
    let temp = NamedTempFile::new_in(parent)
        .map_err(|err| ExtensionError::cache_io("create local staging tempfile", parent, err))?;
    let sha256 = sha256_hex(bytes);
    fs::write(temp.path(), bytes).map_err(|err| {
        ExtensionError::cache_io("write local staging tempfile", temp.path(), err)
    })?;
    Ok(AcquiredBytes {
        temp,
        sha256,
        package_metadata: None,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::io::Write as _;
    use std::path::Path;

    use super::*;
    use crate::manifest::{ExtensionSource, PackageRequest};
    use crate::package::PackageMetadata;
    use crate::test_support::{
        cache_env, cached_bytes, fixed_now, plugin_target_scope, project_scope, scratch_dir, tool,
        write_source,
    };

    #[test]
    fn cache_miss_hit_and_change() {
        let cache_dir = scratch_dir("resolver-local-cache");
        let project_dir = scratch_dir("resolver-local-project");
        let source_dir = scratch_dir("resolver-local-source");
        let first = write_source(&source_dir, "first.wasm", b"first");
        let second = write_source(&source_dir, "second.wasm", b"second");
        let scope = project_scope();
        let first_tool = tool(ExtensionSource::LocalPath(first.clone()), None);

        let _env = cache_env(&cache_dir);

        let resolved =
            resolve(&scope, &first_tool, fixed_now(), &project_dir).expect("cache miss resolves");
        assert_eq!(fs::read(&resolved.bytes_path).expect("cached bytes"), b"first");

        fs::write(&first, b"changed-at-source").expect("mutate source");
        let hit =
            resolve(&scope, &first_tool, fixed_now(), &project_dir).expect("cache hit resolves");
        assert_eq!(hit.bytes_path, resolved.bytes_path);
        assert_eq!(cached_bytes(&scope, &first_tool), b"first");

        let changed_tool = tool(ExtensionSource::LocalPath(second), None);
        let changed = resolve(&scope, &changed_tool, fixed_now(), &project_dir)
            .expect("changed source re-stages");
        assert_eq!(changed.bytes_path, resolved.bytes_path);
        assert_eq!(cached_bytes(&scope, &changed_tool), b"second");
    }

    #[test]
    fn scopes_have_isolated_cache_dirs() {
        let cache_dir = scratch_dir("resolver-scope-cache");
        let project_dir = scratch_dir("resolver-scope-project");
        let source_dir = scratch_dir("resolver-scope-source");
        let capability_dir = scratch_dir("resolver-adapter");
        let source = write_source(&source_dir, "module.wasm", b"same");
        let project = project_scope();
        let adapter = plugin_target_scope(&capability_dir);
        let declared = tool(ExtensionSource::LocalPath(source), None);

        let _env = cache_env(&cache_dir);

        let project_resolved =
            resolve(&project, &declared, fixed_now(), &project_dir).expect("project resolve");
        let adapter_resolved =
            resolve(&adapter, &declared, fixed_now(), &project_dir).expect("adapter resolve");
        assert_ne!(project_resolved.bytes_path, adapter_resolved.bytes_path);
        assert!(project_resolved.bytes_path.to_string_lossy().contains("project--demo"));
        assert!(
            adapter_resolved.bytes_path.to_string_lossy().contains("adapter--target--contracts")
        );
    }

    #[test]
    fn package_source_uses_fetcher() {
        let cache_dir = scratch_dir("resolver-package-cache");
        let project_dir = scratch_dir("resolver-package-project");
        let scope = project_scope();
        let package = PackageRequest {
            namespace: "specify".to_string(),
            name: "contract".to_string(),
            version: "1.0.0".to_string(),
        };
        let declared = tool(ExtensionSource::Package(package), None);

        let bytes: &[u8] = b"package-bytes";
        let calls = Cell::new(0_u32);
        let fetch = |request: &PackageRequest, dest_hint: &Path| {
            calls.set(calls.get() + 1);
            let parent = dest_hint.parent().expect("dest hint has parent");
            fs::create_dir_all(parent).expect("create mock package temp parent");
            let mut temp = NamedTempFile::new_in(parent).expect("create mock package tempfile");
            temp.write_all(bytes).expect("write mock package bytes");
            Ok(AcquiredBytes {
                temp,
                sha256: sha256_hex(bytes),
                package_metadata: Some(PackageMetadata {
                    name: request.name_ref(),
                    version: request.version.clone(),
                    registry: "augentic.io".to_string(),
                }),
            })
        };

        let _env = cache_env(&cache_dir);

        let resolved = resolve_with(&scope, &declared, fixed_now(), &project_dir, fetch)
            .expect("package resolves");
        assert_eq!(fs::read(resolved.bytes_path).expect("cached bytes"), b"package-bytes");
        assert_eq!(calls.get(), 1);

        let sidecar = cache::read_sidecar(
            &cache::sidecar_path(&scope, &declared.name, &declared.version).expect("sidecar path"),
        )
        .expect("read sidecar")
        .expect("sidecar exists");
        assert_eq!(sidecar.source, "specify:contract@1.0.0");
        let sidecar_package = sidecar.package.as_ref().expect("sidecar package");
        assert_eq!(sidecar_package.name.as_str(), "specify:contract");
        assert_eq!(sidecar_package.version.as_str(), "1.0.0");
        assert_eq!(sidecar_package.registry.as_str(), "augentic.io");

        resolve_with(&scope, &declared, fixed_now(), &project_dir, fetch)
            .expect("package cache hit resolves");
        assert_eq!(calls.get(), 1, "cache hit must not fetch package again");
    }

    #[test]
    fn template_source_expands() {
        let cache_dir = scratch_dir("resolver-template-cache");
        let project_dir = scratch_dir("resolver-template-project");
        let wasm_dir = project_dir.join("tools");
        fs::create_dir_all(&wasm_dir).expect("create tools dir");
        fs::write(wasm_dir.join("vectis.wasm"), b"template-bytes").expect("write wasm");

        let scope = project_scope();
        let declared =
            tool(ExtensionSource::TemplatePath("$PROJECT_DIR/tools/vectis.wasm".to_string()), None);

        let _env = cache_env(&cache_dir);

        let resolved =
            resolve(&scope, &declared, fixed_now(), &project_dir).expect("template resolves");
        assert_eq!(fs::read(&resolved.bytes_path).expect("cached bytes"), b"template-bytes");
    }

    #[test]
    fn template_with_parent_segments_resolves() {
        let cache_dir = scratch_dir("resolver-template-parent-cache");
        let project_dir = scratch_dir("resolver-template-parent-project");
        let sibling_dir = project_dir.parent().expect("parent").join("sibling-cli");
        fs::create_dir_all(&sibling_dir).expect("create sibling dir");
        fs::write(sibling_dir.join("vectis.wasm"), b"sibling-bytes").expect("write wasm");

        let scope = project_scope();
        let declared = tool(
            ExtensionSource::TemplatePath("$PROJECT_DIR/../sibling-cli/vectis.wasm".to_string()),
            None,
        );

        let _env = cache_env(&cache_dir);

        let resolved = resolve(&scope, &declared, fixed_now(), &project_dir)
            .expect("parent-segment template resolves");
        assert_eq!(fs::read(&resolved.bytes_path).expect("cached bytes"), b"sibling-bytes");
    }
}
