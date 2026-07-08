//! Parsing the `<adapter>` argument: package references
//! (`specify:<name>@<semver>`), first-party shorthand (`omnia`,
//! `omnia@1.0.0`), and local component paths (`./adapter.wasm`,
//! `file://…/adapter.wasm`).
//!
//! A package reference (and the versioned first-party shorthand, its
//! sugar) is an *immutable*,
//! content-addressed identity with a mandatory exact SemVer pin and no
//! branch or tag defaulting. A package reference resolves from the
//! global content-addressed store entry ([`AdapterUri::from_package`])
//! as a local file; nothing installs into the store today — an
//! install-on-fetch leg lands in-guest.
//!
//! A bare first-party name (`omnia`) is the development shorthand: it
//! resolves the sibling/in-repo release build
//! (`target/wasm32-wasip2/release/<name>.wasm`, built by
//! `cargo make release` in the adapters repo). GitHub URLs are refused —
//! a source checkout no longer yields a usable adapter artifact.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use schema::cache::adapter_store_entry;

use crate::adapter::{AdapterRef, dev_component_paths};

/// Where a parsed `<adapter>` argument's component came from — decides
/// whether init mirrors the file into the project component cache.
///
/// Store entries are read in place (version-pinned, globally shared);
/// development release builds are read live so the adapter dev loop
/// never hits a stale mirror; only an operator's own [`Self::Local`]
/// file is copied into `<project-cache>/components/<name>.wasm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdapterOrigin {
    /// Global content-addressed store entry (package reference).
    Store,
    /// Development release build resolved by the bare-name probe.
    Dev,
    /// Operator-supplied local `.wasm` component file.
    Local,
}

#[derive(Debug)]
pub(super) struct AdapterUri {
    pub(crate) adapter_value: String,
    pub(crate) adapter_name: String,
    /// The resolved component file: a global store entry for package
    /// references, a development release build for bare shorthand, or
    /// the operator's own file for local paths.
    pub(crate) component: PathBuf,
    /// Which probe produced `component` (see [`AdapterOrigin`]).
    pub(crate) origin: AdapterOrigin,
}

impl AdapterUri {
    pub(crate) fn parse(adapter: &str, project_dir: &Path) -> Result<Self, Error> {
        if is_github_url(adapter) {
            return Err(Error::Diag {
                code: "adapter-github-uri-unsupported",
                detail: format!(
                    "GitHub adapter URIs are not supported (`{adapter}`): a source checkout \
                     does not yield a usable adapter artifact. Pin a published component \
                     (`specify:<name>@<semver>`), point at a local `.wasm` component file, or \
                     build the development sibling with `cargo make release`"
                ),
            });
        }
        if let Some(package) = AdapterPackageRef::recognize(adapter) {
            return Self::from_package(&package?);
        }
        if let Some((name, version)) = parse_first_party_shorthand(adapter) {
            return version.map_or_else(
                || Self::from_dev(name, project_dir),
                |version| {
                    Self::from_package(&AdapterPackageRef {
                        namespace: FIRST_PARTY_NAMESPACE.to_string(),
                        name: name.to_string(),
                        version,
                    })
                },
            );
        }
        Self::from_local(adapter, project_dir)
    }

    /// Resolve an immutable [`AdapterPackageRef`] registry locator from
    /// the global content-addressed adapter store.
    ///
    /// Resolve-only: the store entry for the pinned `(name, version)`
    /// must already be present (nothing installs today — the
    /// install-on-fetch leg returns in-guest). A missing entry is
    /// `adapter-package-not-installed` rather than a silent fallback to
    /// a mutable checkout.
    fn from_package(package: &AdapterPackageRef) -> Result<Self, Error> {
        let version = package.version.to_string();
        let component = adapter_store_entry(&package.name, &version);
        if !component.is_file() {
            return Err(Error::Diag {
                code: "adapter-package-not-installed",
                detail: format!(
                    "adapter package `{}` is not installed in the global store at {}; `specify init` installs the component before scaffolding",
                    package.wire_value(),
                    component.display()
                ),
            });
        }
        Ok(Self {
            adapter_value: package.wire_value(),
            adapter_name: package.name.clone(),
            component,
            origin: AdapterOrigin::Store,
        })
    }

    /// Resolve a bare first-party name to its development release
    /// build (`target/wasm32-wasip2/release/<name>.wasm` under
    /// the project or the sibling `specify-adapters` checkout).
    fn from_dev(name: &str, project_dir: &Path) -> Result<Self, Error> {
        let candidates = dev_component_paths(project_dir, name);
        let Some(component) = candidates.iter().find(|path| path.is_file()).cloned() else {
            let probed = candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "bare adapter name `{name}` resolves the development release build, but no \
                     component was found at {probed}; build it with `cargo make release` in the \
                     adapters repo or pin a published version (`specify:{name}@<semver>`)"
                ),
            });
        };
        Ok(Self {
            adapter_value: name.to_string(),
            adapter_name: name.to_string(),
            component,
            origin: AdapterOrigin::Dev,
        })
    }

    fn from_local(adapter: &str, project_dir: &Path) -> Result<Self, Error> {
        let path =
            adapter.strip_prefix("file://").map_or_else(|| PathBuf::from(adapter), PathBuf::from);
        let component = if path.is_absolute() { path } else { project_dir.join(path) };
        ensure_component_file(&component, adapter)?;
        let canonical = fs::canonicalize(&component).map_err(|err| Error::Diag {
            code: "adapter-canonicalize-failed",
            detail: format!(
                "failed to canonicalize local adapter `{adapter}` at {}: {err}",
                component.display()
            ),
        })?;
        let adapter_name = adapter_name_from_component(&canonical)?;
        let adapter_value = format!("file://{}", canonical.display());
        Ok(Self {
            adapter_value,
            adapter_name,
            component: canonical,
            origin: AdapterOrigin::Local,
        })
    }
}

/// An immutable, content-addressed adapter package reference of the
/// form `<namespace>:<name>@<semver>` (e.g. `specify:omnia@1.0.0`).
///
/// The exact SemVer pin is mandatory: there is no branch or tag
/// defaulting, so a reference always names one immutable artifact. The
/// recorded registry content digest backstops a moved tag
/// as `adapter-digest-mismatch` at read time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AdapterPackageRef {
    namespace: String,
    name: String,
    version: semver::Version,
}

impl AdapterPackageRef {
    /// Recognise an adapter package reference `<namespace>:<name>@<semver>`.
    ///
    /// Returns `None` when `adapter` is not a package-ref shape — so
    /// URL schemes (`https://`, `file://`), Windows drive paths
    /// (`C:\…`), bare names, and local paths keep flowing through the
    /// shorthand / local branches. Returns `Some(Err(_))` when the
    /// shape *is* a package reference but the version pin is missing
    /// or not exact SemVer.
    fn recognize(adapter: &str) -> Option<Result<Self, Error>> {
        let (namespace, rest) = adapter.split_once(':')?;
        // `//` after the colon is a URL authority (`https://`,
        // `file://`); a non-kebab namespace (e.g. the `C` of `C:\`) is a
        // drive path. Neither is a package reference.
        if rest.starts_with('/') || !is_first_party_name(namespace) {
            return None;
        }
        Some(Self::parse_validated(namespace, rest, adapter))
    }

    fn parse_validated(namespace: &str, rest: &str, original: &str) -> Result<Self, Error> {
        let (name, version) = rest.split_once('@').ok_or_else(|| Error::Diag {
            code: "adapter-package-ref-version-required",
            detail: format!(
                "adapter package reference `{original}` must pin an exact SemVer version (`{namespace}:<name>@<version>`); there is no branch or tag defaulting"
            ),
        })?;
        if name.is_empty() {
            return Err(Error::Diag {
                code: "adapter-package-ref-malformed",
                detail: format!(
                    "adapter package reference `{original}` is missing a package name before `@`"
                ),
            });
        }
        let version = semver::Version::parse(version).map_err(|err| Error::Diag {
            code: "adapter-package-ref-version-required",
            detail: format!(
                "adapter package reference `{original}` must pin an exact SemVer version, not `{version}`: {err}"
            ),
        })?;
        Ok(Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version,
        })
    }

    /// The canonical `<namespace>:<name>@<version>` wire form recorded
    /// as `project.yaml.adapter`.
    fn wire_value(&self) -> String {
        format!("{}:{}@{}", self.namespace, self.name, self.version)
    }
}

/// Public projection of an adapter package reference for install
/// layers.
///
/// Carries the `(namespace, name, version)` an install leg keys the
/// package pull and the global store entry by (see
/// [`crate::hydrate::Fetch`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterPackage {
    /// First-party package namespace (e.g. `specify`) — the wasm-pkg
    /// namespace under the registry host.
    pub namespace: String,
    /// Kebab-case adapter name.
    pub name: String,
    /// Mandatory exact-SemVer pin.
    pub version: semver::Version,
}

impl AdapterPackage {
    /// A first-party pinned identity — the namespace sugar the
    /// versioned shorthand (`<name>@<semver>`) expands to. Used by the
    /// hydration ref collector to key `plan.yaml` source pins, which
    /// carry only the bare adapter name.
    #[must_use]
    pub fn first_party(name: impl Into<String>, version: semver::Version) -> Self {
        Self {
            namespace: FIRST_PARTY_NAMESPACE.to_string(),
            name: name.into(),
            version,
        }
    }
}

/// Recognise an adapter argument the root install layer must fetch
/// before scaffolding.
///
/// Matches a `<namespace>:<name>@<semver>` package reference, or the
/// versioned first-party shorthand (`omnia@1.0.0` — sugar for
/// `specify:omnia@1.0.0`).
///
/// Returns `None` for non-package shapes (bare names, paths, URLs), so
/// those keep flowing through the dev / local branches; `Some(Err(_))`
/// when the shape *is* a package reference but the SemVer pin is
/// missing or malformed.
#[must_use]
pub fn recognize_package(value: &str) -> Option<Result<AdapterPackage, Error>> {
    if let Some(parsed) = AdapterPackageRef::recognize(value) {
        return Some(parsed.map(|package| AdapterPackage {
            namespace: package.namespace,
            name: package.name,
            version: package.version,
        }));
    }
    match parse_first_party_shorthand(value) {
        Some((name, Some(version))) => Some(Ok(AdapterPackage {
            namespace: FIRST_PARTY_NAMESPACE.to_string(),
            name: name.to_string(),
            version,
        })),
        Some((_, None)) | None => None,
    }
}

fn is_github_url(adapter: &str) -> bool {
    adapter.starts_with("https://github.com/")
}

/// The wasm-pkg namespace first-party adapters publish under
/// (`specify:<name>@<semver>` via `wkg publish` in the adapters repo;
/// specify: naming cut — `augentic:` is reserved, not routed).
const FIRST_PARTY_NAMESPACE: &str = "specify";

/// Recognise a first-party adapter shorthand and split it into
/// `(name, version)`. A bare `name` carries no pin (`None`) and
/// resolves the development release build; a `name@<semver>` carries
/// the parsed [`semver::Version`] and is sugar for
/// the `specify:<name>@<semver>` package reference. Returns `None`
/// for paths (`./foo`, `/abs`, `file://…`) and URLs (anything carrying
/// `:` or `/`), and for a `@suffix` that is not exact semver — so
/// those keep flowing through [`AdapterUri::from_local`].
fn parse_first_party_shorthand(adapter: &str) -> Option<(&str, Option<semver::Version>)> {
    if adapter.contains('/') || adapter.contains(':') {
        return None;
    }
    match adapter.split_once('@') {
        Some((name, reference)) if is_first_party_name(name) => {
            let version = semver::Version::parse(reference).ok()?;
            Some((name, Some(version)))
        }
        None if is_first_party_name(adapter) => Some((adapter, None)),
        Some(_) | None => None,
    }
}

/// `^[a-z][a-z0-9-]*$` — a kebab-case first-party adapter name.
fn is_first_party_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn ensure_component_file(path: &Path, original: &str) -> Result<(), Error> {
    if path.is_file() && path.extension().is_some_and(|ext| ext == "wasm") {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-component-missing",
        detail: format!(
            "adapter `{original}` did not resolve to a `.wasm` component file at {} (an \
             adapter is a single WebAssembly component)",
            path.display()
        ),
    })
}

/// Derive the kebab-case adapter name from a component filename:
/// `specify_intent.wasm` → `intent`, `my-adapter.wasm` → `my-adapter`
/// (the cargo `specify_` artifact prefix is stripped and underscores
/// fold to kebab dashes).
fn adapter_name_from_component(path: &Path) -> Result<String, Error> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| Error::Diag {
        code: "adapter-dir-name-unresolved",
        detail: format!("cannot derive adapter name from {}", path.display()),
    })?;
    let stem =
        stem.strip_prefix("specify_").or_else(|| stem.strip_prefix("specify-")).unwrap_or(stem);
    Ok(stem.replace('_', "-"))
}

/// Extract the kebab-case adapter name from a `project.yaml.adapter`
/// value. Accepts:
///
/// - bare kebab names (`omnia`) — returned unchanged,
/// - package references (`specify:omnia@1.0.0`) — the `<name>`
///   between `:` and `@`,
/// - `file://` URIs — last path component, `.wasm` suffix and cargo
///   artifact prefix stripped,
/// - bare local paths — same treatment.
#[must_use]
pub fn adapter_name_from_value(value: &str) -> String {
    let stripped = strip_at_ref_suffix(value);
    let stripped = stripped.strip_prefix("file://").unwrap_or(stripped);
    let stripped = stripped.strip_suffix('/').unwrap_or(stripped);
    let stripped = package_ref_name(stripped).unwrap_or(stripped);
    let last = stripped.rsplit('/').next().unwrap_or(stripped);
    let last = last.strip_suffix(".wasm").unwrap_or(last);
    let last =
        last.strip_prefix("specify_").or_else(|| last.strip_prefix("specify-")).unwrap_or(last);
    last.replace('_', "-")
}

/// If `value` is a bare package reference `<namespace>:<name>` (kebab
/// namespace, no `//` URL authority), return the `<name>`. Otherwise
/// `None`, so URLs and drive paths keep their path-component handling.
fn package_ref_name(value: &str) -> Option<&str> {
    let (namespace, rest) = value.split_once(':')?;
    (!rest.starts_with('/') && is_first_party_name(namespace)).then_some(rest)
}

/// Build an [`AdapterRef`] identity from a `project.yaml.adapter` (or
/// slice `target`) value: the kebab `name` plus an optional pinned
/// semver `version` recovered from the `@<suffix>`.
///
/// The version is `Some(_)` only when the `@suffix` parses as exact
/// semver — a bare name or a `file://` path yields `version: None`, so
/// resolution falls back to the development artifact.
#[must_use]
pub fn adapter_ref_from_value(value: &str) -> AdapterRef {
    let name = adapter_name_from_value(value);
    let version = at_ref_suffix(value).and_then(|suffix| semver::Version::parse(suffix).ok());
    AdapterRef { name, version }
}

fn strip_at_ref_suffix(value: &str) -> &str {
    let last_slash = value.rfind('/').unwrap_or(0);
    if let Some(at) = value.rfind('@')
        && at > last_slash
    {
        return &value[..at];
    }
    value
}

/// The `@<suffix>` after the last path segment, if any — the inverse of
/// [`strip_at_ref_suffix`].
fn at_ref_suffix(value: &str) -> Option<&str> {
    let last_slash = value.rfind('/').unwrap_or(0);
    let at = value.rfind('@')?;
    (at > last_slash && at + 1 < value.len()).then(|| &value[at + 1..])
}

#[cfg(test)]
mod tests;
