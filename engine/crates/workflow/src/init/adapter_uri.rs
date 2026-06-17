//! Parsing the `<adapter>` argument: first-party shorthand
//! (`omnia`, `omnia@1.0.0`), package references
//! (`specify:<name>@<semver>`), bare local paths, `file://` URIs, and
//! `https://github.com/...` URIs (with optional `@ref` or
//! `tree/<ref>` discriminators).
//!
//! First-party shorthand resolves a bare adapter name to the canonical
//! published adapter on GitHub. The shorthand carries the RFC-47 semver
//! identity (`omnia@1.0.0`); the git checkout ref is derived as
//! `v<major>` while `project.yaml.adapter` records the full semver pin.
//!
//! A package reference (`<namespace>:<name>@<semver>`, e.g.
//! `specify:omnia@1.2.0`) is the RFC-48 D2 registry locator: an
//! *immutable*, content-addressed identity with a mandatory exact
//! SemVer pin and no branch or tag defaulting. The root `specify init`
//! layer installs the pinned artifact into the global content-addressed
//! store ([`recognize_package`] → `registry::store::install_tofu`) before
//! this flow runs, so a package reference resolves from the store entry
//! ([`AdapterUri::from_package`]) as a local source.

use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_schema::cache::adapter_store_entry;
use tempfile::TempDir;

use crate::adapter::AdapterRef;
use crate::init::git::sparse_checkout_github;

#[derive(Debug)]
pub(super) struct AdapterUri {
    pub(crate) adapter_value: String,
    pub(crate) adapter_name: String,
    pub(crate) source_dir: PathBuf,
    /// `true` when `source_dir` is a global content-addressed store entry
    /// (RFC-48 D5 package reference). The init manifest-cache copy is
    /// skipped for these: the resolver reads the store entry in place,
    /// version-pinned, so there is nothing to mirror per project.
    pub(crate) from_store: bool,
    _checkout_guard: Option<TempDir>,
}

impl AdapterUri {
    pub(crate) fn parse(adapter: &str, project_dir: &Path) -> Result<Self, Error> {
        if is_github_url(adapter) {
            return Self::from_github(adapter);
        }
        if let Some(package) = AdapterPackageRef::recognize(adapter) {
            return Self::from_package(&package?);
        }
        if let Some((name, version)) = parse_first_party_shorthand(adapter) {
            return Self::from_shorthand(name, version.as_ref());
        }
        Self::from_local(adapter, project_dir)
    }

    /// Resolve an immutable [`AdapterPackageRef`] registry locator from
    /// the global content-addressed adapter store (RFC-48 D5).
    ///
    /// The artifact is installed into the store by the root `specify init`
    /// layer (`registry::store::install_tofu`) before this workflow flow
    /// runs, so the store entry for the pinned `(name, version)` is
    /// already present and this resolves it as a local source. A missing
    /// entry is `adapter-package-not-installed` — the install step did not
    /// run or failed — rather than a silent fallback to a mutable
    /// checkout.
    fn from_package(package: &AdapterPackageRef) -> Result<Self, Error> {
        let version = package.version.to_string();
        let source_dir = adapter_store_entry(&package.name, &version);
        if !source_dir.join(crate::adapter::ADAPTER_FILENAME).is_file() {
            return Err(Error::Diag {
                code: "adapter-package-not-installed",
                detail: format!(
                    "adapter package `{}` is not installed in the global store at {}; `specify init` installs the artifact before scaffolding (RFC-48 D5)",
                    package.wire_value(),
                    source_dir.display()
                ),
            });
        }
        Ok(Self {
            adapter_value: package.wire_value(),
            adapter_name: package.name.clone(),
            source_dir,
            from_store: true,
            _checkout_guard: None,
        })
    }

    fn from_local(adapter: &str, project_dir: &Path) -> Result<Self, Error> {
        let path =
            adapter.strip_prefix("file://").map_or_else(|| PathBuf::from(adapter), PathBuf::from);
        let source_dir = if path.is_absolute() { path } else { project_dir.join(path) };
        Self::from_resolved_dir(&source_dir, adapter)
    }

    /// Build an [`AdapterUri`] from an already-resolved local directory,
    /// recording a `file://` `adapter_value`. Used by [`Self::from_local`]
    /// for bare and `file://` paths.
    fn from_resolved_dir(source_dir: &Path, original: &str) -> Result<Self, Error> {
        ensure_adapter_dir(source_dir, original)?;
        let canonical = fs::canonicalize(source_dir).map_err(|err| Error::Diag {
            code: "adapter-canonicalize-failed",
            detail: format!(
                "failed to canonicalize local adapter `{original}` at {}: {err}",
                source_dir.display()
            ),
        })?;
        let adapter_name = adapter_name_from_dir(&canonical)?;
        let adapter_value = format!("file://{}", canonical.display());
        Ok(Self {
            adapter_value,
            adapter_name,
            source_dir: canonical,
            from_store: false,
            _checkout_guard: None,
        })
    }

    /// Resolve a first-party shorthand (`omnia`, `omnia@1.0.0`) to the
    /// canonical published adapter on GitHub. `init` is target-only, so
    /// the shorthand resolves under `adapters/targets/<name>/`.
    ///
    /// The git checkout ref is derived from the pinned semver as
    /// `v<major>` (defaulting to `v1` when no version is given), since
    /// transport stays git-based until RFC-48. `adapter_value` records
    /// the canonical `name@<semver>` identity (RFC-47), not the derived
    /// checkout URL, so `project.yaml.adapter` carries the version pin.
    fn from_shorthand(name: &str, version: Option<&semver::Version>) -> Result<Self, Error> {
        let git_ref = version.map_or_else(|| "v1".to_string(), |v| format!("v{}", v.major));
        let repo = first_party_repo(name);
        let url = format!("https://github.com/augentic/{repo}/adapters/targets/{name}@{git_ref}");
        let mut uri = Self::from_github(&url)?;
        uri.adapter_value =
            version.map_or_else(|| name.to_string(), |version| format!("{name}@{version}"));
        Ok(uri)
    }

    fn from_github(adapter: &str) -> Result<Self, Error> {
        let spec = GithubAdapterUri::parse(adapter)?;
        let repo_url = format!("https://github.com/{}/{}.git", spec.owner, spec.repo);
        let checkout =
            sparse_checkout_github(&repo_url, spec.checkout_ref.as_deref(), &spec.adapter_path)?;
        let source_dir = checkout.path().join(&spec.adapter_path);
        ensure_adapter_dir(&source_dir, adapter)?;

        Ok(Self {
            adapter_value: adapter.to_string(),
            adapter_name: spec.adapter_name,
            source_dir,
            from_store: false,
            _checkout_guard: Some(checkout),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct GithubAdapterUri {
    owner: String,
    repo: String,
    checkout_ref: Option<String>,
    adapter_path: String,
    adapter_name: String,
}

impl GithubAdapterUri {
    fn parse(adapter: &str) -> Result<Self, Error> {
        let (without_suffix, suffix_ref) = split_ref_suffix(adapter);
        let pathless =
            without_suffix.strip_prefix("https://github.com/").ok_or_else(|| Error::Diag {
                code: "adapter-github-uri-unsupported",
                detail: format!("unsupported GitHub adapter URI `{adapter}`"),
            })?;
        let mut parts: Vec<&str> = pathless.split('/').filter(|part| !part.is_empty()).collect();
        if parts.len() < 3 {
            return Err(Error::Diag {
                code: "adapter-github-uri-malformed",
                detail: format!(
                    "GitHub adapter URI `{adapter}` must include owner, repo, and adapter path"
                ),
            });
        }
        let owner = parts.remove(0).to_string();
        let repo = parts.remove(0).to_string();

        let (tree_ref, adapter_parts): (Option<&str>, Vec<&str>) = if parts.first() == Some(&"tree")
        {
            if parts.len() < 3 {
                return Err(Error::Diag {
                    code: "adapter-github-uri-malformed",
                    detail: format!(
                        "GitHub tree adapter URI `{adapter}` must include a ref and adapter path"
                    ),
                });
            }
            (Some(parts[1]), parts[2..].to_vec())
        } else {
            (None, parts)
        };

        let checkout_ref = suffix_ref.or(tree_ref).map(str::to_string);
        let adapter_path = adapter_parts.join("/");
        let adapter_name = adapter_parts.last().ok_or_else(|| Error::Diag {
            code: "adapter-url-name-unresolved",
            detail: format!("cannot derive a adapter name from `{adapter}`"),
        })?;

        Ok(Self {
            owner,
            repo,
            checkout_ref,
            adapter_path,
            adapter_name: (*adapter_name).to_string(),
        })
    }
}

/// An immutable, content-addressed adapter package reference of the
/// form `<namespace>:<name>@<semver>` (e.g. `specify:omnia@1.2.0`) — the
/// RFC-48 D2 registry locator.
///
/// The exact SemVer pin is mandatory: there is no branch or tag
/// defaulting, so a reference always names one immutable artifact. The
/// recorded registry content digest (RFC-48 D4) backstops a moved tag
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
    /// GitHub / shorthand / local branches. Returns `Some(Err(_))` when
    /// the shape *is* a package reference but the version pin is missing
    /// or not exact SemVer (RFC-48 D2 forbids branch/tag defaulting).
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

/// Public projection of an adapter package reference for the root layer.
///
/// Carries the `(namespace, name, version)` the root `specify init`
/// install layer derives an OCI reference from
/// (`registry::oci::adapter_reference`) and keys the global store entry by
/// (`registry::store::install_tofu`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterPackage {
    /// First-party package namespace (e.g. `specify`) — the OCI repo
    /// path segment under the registry host.
    pub namespace: String,
    /// Kebab-case adapter name.
    pub name: String,
    /// Mandatory exact-SemVer pin (RFC-48 D2).
    pub version: semver::Version,
}

/// Recognise a `<namespace>:<name>@<semver>` adapter package reference so
/// the root install layer can fetch it before scaffolding.
///
/// Returns `None` for non-package shapes (shorthand, paths, URLs), so
/// those keep flowing through the git / local branches; `Some(Err(_))`
/// when the shape *is* a package reference but the SemVer pin is missing
/// or malformed (RFC-48 D2 forbids branch/tag defaulting).
#[must_use]
pub fn recognize_package(value: &str) -> Option<Result<AdapterPackage, Error>> {
    AdapterPackageRef::recognize(value).map(|parsed| {
        parsed.map(|package| AdapterPackage {
            namespace: package.namespace,
            name: package.name,
            version: package.version,
        })
    })
}

fn is_github_url(adapter: &str) -> bool {
    adapter.starts_with("https://github.com/")
}

/// The `augentic/<repo>` segment hosting a first-party adapter's
/// sparse-checkout source.
///
/// Adapters that bundle a WASI extension have extracted to
/// `augentic/specify-adapters` (RFC-48 D7/D10, RFC-49 T6); the remaining
/// prose-only adapters still live in the platform repo until the rest of
/// the topology migration lands. This git shorthand is itself
/// transitional — the durable first-party resolution path is the RFC-48
/// D2 registry locator (`specify:<name>@<semver>`), which retires the
/// per-name routing once registry transport is wired.
fn first_party_repo(name: &str) -> &'static str {
    match name {
        "contracts" | "vectis" => "specify-adapters",
        _ => "specify",
    }
}

/// Recognise a first-party adapter shorthand and split it into
/// `(name, version)`. A bare `name` carries no pin (`None`); a
/// `name@<semver>` carries the parsed [`semver::Version`] (RFC-47
/// identity). Returns `None` for paths (`./foo`, `/abs`, `file://…`)
/// and URLs (anything carrying `:` or `/`), and for a `@suffix` that is
/// not exact semver — so those keep flowing through
/// [`AdapterUri::from_local`] / [`AdapterUri::from_github`].
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

fn split_ref_suffix(adapter: &str) -> (&str, Option<&str>) {
    let last_slash = adapter.rfind('/').unwrap_or(0);
    if let Some(at) = adapter.rfind('@')
        && at > last_slash
        && at + 1 < adapter.len()
    {
        return (&adapter[..at], Some(&adapter[at + 1..]));
    }
    (adapter, None)
}

fn ensure_adapter_dir(path: &Path, original: &str) -> Result<(), Error> {
    if path.join(crate::adapter::ADAPTER_FILENAME).is_file() {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-dir-missing-manifest",
        detail: format!(
            "adapter `{original}` did not resolve to a directory with `{}` at {}",
            crate::adapter::ADAPTER_FILENAME,
            path.display()
        ),
    })
}

fn adapter_name_from_dir(path: &Path) -> Result<String, Error> {
    path.file_name().and_then(|name| name.to_str()).map(str::to_string).ok_or_else(|| Error::Diag {
        code: "adapter-dir-name-unresolved",
        detail: format!("cannot derive adapter name from {}", path.display()),
    })
}

/// Extract the kebab-case adapter name from a `project.yaml.adapter`
/// value. Accepts:
///
/// - bare kebab names (`omnia`) — returned unchanged,
/// - package references (`specify:omnia@1.2.0`) — the `<name>` between
///   `:` and `@`,
/// - `file://` URIs — last path component,
/// - `https://...` URIs — last path component (suffix `@ref` stripped),
/// - bare local paths — last path component.
#[must_use]
pub fn adapter_name_from_value(value: &str) -> &str {
    let stripped = strip_at_ref_suffix(value);
    let stripped = stripped.strip_prefix("file://").unwrap_or(stripped);
    let stripped = stripped.strip_suffix('/').unwrap_or(stripped);
    let stripped = package_ref_name(stripped).unwrap_or(stripped);
    stripped.rsplit('/').next().unwrap_or(stripped)
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
/// semver `version` recovered from the `@<suffix>` (RFC-47 D2).
///
/// The version is `Some(_)` only when the `@suffix` parses as exact
/// semver — a bare name, a `file://` path, or a GitHub URL carrying a
/// non-semver git ref (e.g. `@v1`) all yield `version: None`, so
/// resolution falls back to the single installed identity.
#[must_use]
pub fn adapter_ref_from_value(value: &str) -> AdapterRef {
    let name = adapter_name_from_value(value).to_string();
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
