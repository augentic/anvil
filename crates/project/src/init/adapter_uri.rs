//! Parsing the `<adapter>` argument: package references
//! (`specify:<name>@<semver>`), first-party shorthand (`omnia`,
//! `omnia@1.0.0`), and local component paths (`./adapter.wasm`,
//! `file://…/adapter.wasm`).
//!
//! Exact package pins resolve from the global store; bare names defer to
//! the injected resolver (linked Rust in the native harness, release
//! components in the shipped path). GitHub URLs are refused.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use schema::cache::adapter_store_entry;

use crate::adapter::AdapterRef;

/// Where a parsed `<adapter>` argument came from — decides whether
/// init mirrors a file into the project component cache.
///
/// Store entries are read in place (version-pinned, globally shared);
/// bare development names carry no component at parse time (the
/// injected `Resolver` locates one, or a linked native adapter); only
/// an operator's own [`Self::Local`] file is copied into
/// `<project-cache>/components/<name>.wasm`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdapterOrigin {
    /// Global content-addressed store entry (package reference).
    Store(PathBuf),
    /// Bare development shorthand — component resolution is deferred
    /// to the injected `Resolver`, so no artifact is demanded here.
    Dev,
    /// Operator-supplied local `.wasm` component file (canonical).
    Local(PathBuf),
}

#[derive(Debug)]
pub(super) struct AdapterUri {
    pub(crate) adapter_value: String,
    pub(crate) adapter_name: String,
    /// Which branch recognised the argument, carrying the parse-time
    /// component file where one exists (see [`AdapterOrigin`]).
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
                     build the in-repo development artifact (`cargo build --release --target \
                     wasm32-wasip2`)"
                ),
            });
        }
        if let Some(package) = AdapterPackageRef::recognize(adapter) {
            return Self::from_package(&package?);
        }
        if let Some((name, version)) = parse_shorthand(adapter) {
            return version.map_or_else(
                || Ok(Self::from_dev(name)),
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
            origin: AdapterOrigin::Store(component),
        })
    }

    /// Defer a bare development identity to the injected `Resolver`.
    ///
    /// The resolver locates
    /// one downstream (the project component cache, then the project's
    /// in-repo release build in the shipped path; linked Rust
    /// crates in the native harness) and raises `adapter-not-found`
    /// with build/install remediation on a miss.
    fn from_dev(name: &str) -> Self {
        Self {
            adapter_value: name.to_string(),
            adapter_name: name.to_string(),
            origin: AdapterOrigin::Dev,
        }
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
        let adapter_name = name_from_component(&canonical)?;
        let adapter_value = format!("file://{}", canonical.display());
        Ok(Self {
            adapter_value,
            adapter_name,
            origin: AdapterOrigin::Local(canonical),
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

fn is_github_url(adapter: &str) -> bool {
    adapter.starts_with("https://github.com/")
}

/// A recognised pinned package identity — the hydration key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PinnedPackage {
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) version: semver::Version,
}

/// Recognise a pinned package identity in an `<adapter>` argument or
/// recorded `project.yaml.adapter` value: a package reference
/// (`specify:omnia@1.0.0`) or the first-party shorthand sugar
/// (`omnia@1.0.0`). Bare names, paths, and URLs are `None` — nothing
/// to hydrate.
pub(super) fn pinned_package(value: &str) -> Option<PinnedPackage> {
    if let Some(Ok(package)) = AdapterPackageRef::recognize(value) {
        return Some(PinnedPackage {
            namespace: package.namespace,
            name: package.name,
            version: package.version,
        });
    }
    if let Some((name, Some(version))) = parse_shorthand(value) {
        return Some(PinnedPackage {
            namespace: FIRST_PARTY_NAMESPACE.to_string(),
            name: name.to_string(),
            version,
        });
    }
    None
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
fn parse_shorthand(adapter: &str) -> Option<(&str, Option<semver::Version>)> {
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
fn name_from_component(path: &Path) -> Result<String, Error> {
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
pub(super) fn adapter_name_from_value(value: &str) -> String {
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

impl AdapterRef {
    /// Build an [`AdapterRef`] identity from a `project.yaml.adapter`
    /// (or slice `target`) value: the kebab `name` plus an optional
    /// pinned semver `version` recovered from the `@<suffix>`.
    ///
    /// The version is `Some(_)` only when the `@suffix` parses as
    /// exact semver — a bare name or a `file://` path yields
    /// `version: None`, so resolution falls back to the development
    /// artifact. (Housed here rather than beside the type because the
    /// value grammar it inverts is this module's.)
    #[must_use]
    pub fn from_value(value: &str) -> Self {
        let name = adapter_name_from_value(value);
        let version = at_ref_suffix(value).and_then(|suffix| semver::Version::parse(suffix).ok());
        Self { name, version }
    }
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
