//! wasm-pkg package resolution for declared tools.

use std::path::Path;

use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use wasm_pkg_client::{Client, Config, PackageRef, Registry, RegistryMapping, Version};

use crate::error::ExtensionError;
use crate::manifest::{PackageRequest, WASM_PKG_CONFIG_PATH};

const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
const FIRST_PARTY_NAMESPACE: &str = "specify";
const FIRST_PARTY_REGISTRY: &str = "augentic.io";

/// Informational package metadata recorded in `meta.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PackageMetadata {
    /// Package name without the version suffix.
    pub name: String,
    /// Exact package version.
    pub version: String,
    /// Registry host used for resolution.
    pub registry: String,
}

/// Bytes acquired from a tool source, ready for digest validation and
/// installation into the cache. Every source streams to a sibling
/// `NamedTempFile` so the install step is a uniform `persist` regardless of
/// whether the bytes came from a local file, an HTTPS download, or a
/// package registry.
#[derive(Debug)]
pub struct AcquiredBytes {
    pub temp: NamedTempFile,
    pub sha256: String,
    pub package_metadata: Option<PackageMetadata>,
}

impl AcquiredBytes {
    pub fn len(&self) -> Result<u64, ExtensionError> {
        self.temp
            .as_file()
            .metadata()
            .map(|m| m.len())
            .map_err(|err| ExtensionError::cache_io("stat staged tool body", self.temp.path(), err))
    }
}

/// Atomically move `temp` over `dest`, mapping the typed `tempfile`
/// persist error into the cache-error vocabulary the resolver speaks.
/// Free function (rather than a method on [`AcquiredBytes`]) so the
/// resolver can destructure the bytes once and move `temp` and
/// `package_metadata` independently without forcing a clone.
pub fn persist_temp(temp: NamedTempFile, dest: &Path) -> Result<(), ExtensionError> {
    temp.persist(dest).map(|_| ()).map_err(|err| {
        ExtensionError::atomic_move_failed(
            err.file.path().to_path_buf(),
            dest.to_path_buf(),
            err.error,
        )
    })
}

/// Fetch package content into a sibling tempfile below `dest_hint`.
///
/// # Errors
///
/// Returns package resolution, registry, stream, or cache staging errors.
pub fn fetch(
    project_dir: &Path, request: &PackageRequest, dest_hint: &Path,
) -> Result<AcquiredBytes, ExtensionError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| ExtensionError::package(request, format!("create tokio runtime: {err}")))?;
    runtime.block_on(fetch_async(request, dest_hint, Some(project_dir)))
}

async fn fetch_async(
    request: &PackageRequest, dest_hint: &Path, project_dir: Option<&Path>,
) -> Result<AcquiredBytes, ExtensionError> {
    let temp_parent = dest_hint.parent().ok_or_else(|| {
        ExtensionError::cache_root(format!(
            "tool package destination has no parent: {}",
            dest_hint.display()
        ))
    })?;
    std::fs::create_dir_all(temp_parent).map_err(|err| {
        ExtensionError::cache_io("create package download staging parent", temp_parent, err)
    })?;
    let temp = NamedTempFile::new_in(temp_parent).map_err(|err| {
        ExtensionError::cache_io("create package download tempfile", temp_parent, err)
    })?;

    let package: PackageRef = request
        .name_ref()
        .parse()
        .map_err(|err| ExtensionError::package(request, format!("parse package name: {err}")))?;
    let version = Version::parse(&request.version)
        .map_err(|err| ExtensionError::package(request, format!("parse package version: {err}")))?;
    let config = load_config(&package, project_dir).await?;
    let resolved_registry = match config.resolve_registry(&package).cloned() {
        Some(registry) => registry,
        None => first_party_registry(&package)?,
    };
    let registry_string = resolved_registry.to_string();
    let client = Client::new(config);
    let release = client.get_release(&package, &version).await.map_err(|err| {
        ExtensionError::package(request, format!("resolve package release: {err}"))
    })?;
    let mut stream = client.stream_content(&package, &release).await.map_err(|err| {
        ExtensionError::package(request, format!("open package content stream: {err}"))
    })?;
    let mut file = tokio::fs::File::from_std(temp.reopen().map_err(|err| {
        ExtensionError::cache_io("open package download tempfile", temp.path(), err)
    })?);
    let mut hasher = specify_schema::digest::Hasher::new();
    let mut total = 0_u64;
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|err| ExtensionError::package(request, format!("stream package content: {err}")))?
    {
        total = total.saturating_add(chunk.len() as u64);
        if total > MAX_PACKAGE_BYTES {
            return Err(ExtensionError::network_too_large(
                request.to_wire_string(),
                MAX_PACKAGE_BYTES,
                Some(total),
            ));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|err| {
            ExtensionError::cache_io("write package download tempfile", temp.path(), err)
        })?;
    }
    file.flush().await.map_err(|err| {
        ExtensionError::cache_io("flush package download tempfile", temp.path(), err)
    })?;
    file.sync_all().await.map_err(|err| {
        ExtensionError::cache_io("sync package download tempfile", temp.path(), err)
    })?;
    drop(file);

    Ok(AcquiredBytes {
        temp,
        sha256: hasher.finalize_hex(),
        package_metadata: Some(PackageMetadata {
            name: request.name_ref(),
            version: request.version.clone(),
            registry: registry_string,
        }),
    })
}

/// Build the wasm-pkg [`Config`] used to resolve `package`, layering
/// (in order, last write wins per key):
///
/// 1. The wasm-pkg global defaults (`Config::global_defaults`).
/// 2. The project-local `.specify/wasm-pkg.toml` when `project_dir` is
///    `Some` and the file exists.
/// 3. The `WKG_CONFIG` override file when the env var is set.
/// 4. An embedded `specify -> augentic.io` namespace mapping when no
///    explicit `specify` mapping was supplied by any layer above.
async fn load_config(
    package: &PackageRef, project_dir: Option<&Path>,
) -> Result<Config, ExtensionError> {
    let mut config = Config::global_defaults().await.map_err(|err| {
        ExtensionError::package_label(package.to_string(), format!("load wasm-pkg config: {err}"))
    })?;

    if let Some(dir) = project_dir {
        let project_config_path = dir.join(WASM_PKG_CONFIG_PATH);
        if project_config_path.is_file() {
            let project_config = Config::from_file(&project_config_path).await.map_err(|err| {
                ExtensionError::package_label(
                    package.to_string(),
                    format!(
                        "load project wasm-pkg config {}: {err}",
                        project_config_path.display()
                    ),
                )
            })?;
            config.merge(project_config);
        }
    }

    if let Some(path) = std::env::var_os("WKG_CONFIG") {
        let override_config = Config::from_file(&path).await.map_err(|err| {
            ExtensionError::package_label(
                package.to_string(),
                format!("load WKG_CONFIG {}: {err}", Path::new(&path).display()),
            )
        })?;
        config.merge(override_config);
    }

    if package.namespace().to_string() == FIRST_PARTY_NAMESPACE
        && config.namespace_registry(package.namespace()).is_none()
    {
        let registry = first_party_registry(package)?;
        config.set_namespace_registry(
            package.namespace().clone(),
            RegistryMapping::Registry(registry),
        );
    }
    Ok(config)
}

fn first_party_registry(package: &PackageRef) -> Result<Registry, ExtensionError> {
    FIRST_PARTY_REGISTRY.parse().map_err(|err| {
        ExtensionError::package_label(
            package.to_string(),
            format!("parse first-party registry default: {err}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::test_support::{EnvGuard, env_lock, scratch_dir};

    fn package_ref() -> PackageRef {
        "specify:contract".parse().expect("parse first-party package ref")
    }

    fn third_party_ref() -> PackageRef {
        "ba:demo".parse().expect("parse third-party package ref")
    }

    /// Point `HOME` and `XDG_CONFIG_HOME` at a fresh scratch dir so
    /// `Config::global_defaults` cannot pull in a developer's personal
    /// `~/.config/wasm-pkg/config.toml` and surprise the assertions.
    fn isolate_global_config_dir(label: &str) -> (PathBuf, [EnvGuard; 2]) {
        let home = scratch_dir(label);
        let guards = [
            EnvGuard::scoped("HOME", Some(&home)),
            EnvGuard::scoped("XDG_CONFIG_HOME", Some(&home)),
        ];
        (home, guards)
    }

    #[test]
    fn embedded_injects_first_party() {
        let _guard = env_lock();
        let (_home, _isolated) = isolate_global_config_dir("package-embedded-default");
        let _wkg = EnvGuard::scoped("WKG_CONFIG", None);

        let package = package_ref();
        let runtime =
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let config = runtime.block_on(load_config(&package, None)).expect("load_config ok");
        let resolved = config.resolve_registry(&package).expect("specify namespace mapped");
        assert_eq!(resolved.to_string(), FIRST_PARTY_REGISTRY);
    }

    #[test]
    fn embedded_skipped_for_others() {
        let _guard = env_lock();
        let (_home, _isolated) = isolate_global_config_dir("package-embedded-other");
        let _wkg = EnvGuard::scoped("WKG_CONFIG", None);

        let package = third_party_ref();
        let runtime =
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let config = runtime.block_on(load_config(&package, None)).expect("load_config ok");
        // wasm-pkg ships a `ba -> bytecodealliance.org` hard-coded
        // fallback; the contract here is only that we did NOT inject
        // `augentic.io` for a non-`specify` namespace.
        let resolved = config.resolve_registry(&package).map(ToString::to_string);
        assert_ne!(resolved.as_deref(), Some(FIRST_PARTY_REGISTRY));
    }

    #[test]
    fn local_config_overrides_default() {
        let _guard = env_lock();
        let (_home, _isolated) = isolate_global_config_dir("package-project-config-home");
        let _wkg = EnvGuard::scoped("WKG_CONFIG", None);

        let project = scratch_dir("package-project-config");
        fs::create_dir_all(project.join(".specify")).expect("create .specify");
        fs::write(
            project.join(WASM_PKG_CONFIG_PATH),
            "[namespace_registries]\nspecify = \"mirror.example.com\"\n",
        )
        .expect("write project wasm-pkg.toml");

        let package = package_ref();
        let runtime =
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let config = runtime
            .block_on(load_config(&package, Some(project.as_path())))
            .expect("load_config ok");
        let resolved = config.resolve_registry(&package).expect("namespace mapped");
        assert_eq!(resolved.to_string(), "mirror.example.com");
    }

    #[test]
    fn wkg_config_overrides_project_local() {
        let _guard = env_lock();
        let (_home, _isolated) = isolate_global_config_dir("package-wkg-overrides-home");

        let project = scratch_dir("package-wkg-overrides-project");
        fs::create_dir_all(project.join(".specify")).expect("create .specify");
        fs::write(
            project.join(WASM_PKG_CONFIG_PATH),
            "[namespace_registries]\nspecify = \"mirror.example.com\"\n",
        )
        .expect("write project wasm-pkg.toml");

        let wkg_config = project.join("wkg-override.toml");
        fs::write(&wkg_config, "[namespace_registries]\nspecify = \"override.example.com\"\n")
            .expect("write wkg override");
        let _wkg = EnvGuard::scoped("WKG_CONFIG", Some(&wkg_config));

        let package = package_ref();
        let runtime =
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let config = runtime
            .block_on(load_config(&package, Some(project.as_path())))
            .expect("load_config ok");
        let resolved = config.resolve_registry(&package).expect("namespace mapped");
        assert_eq!(resolved.to_string(), "override.example.com");
    }

    #[test]
    fn missing_config_skipped() {
        let _guard = env_lock();
        let (_home, _isolated) = isolate_global_config_dir("package-missing-project-home");
        let _wkg = EnvGuard::scoped("WKG_CONFIG", None);

        let project = scratch_dir("package-missing-project-config");
        // Intentionally do not create `.specify/wasm-pkg.toml`.

        let package = package_ref();
        let runtime =
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let config = runtime
            .block_on(load_config(&package, Some(project.as_path())))
            .expect("load_config ok");
        let resolved = config.resolve_registry(&package).expect("specify namespace mapped");
        assert_eq!(resolved.to_string(), FIRST_PARTY_REGISTRY);
    }
}
