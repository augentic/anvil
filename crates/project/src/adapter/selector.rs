//! [`AdapterSelector`] — the typed, kind-preserving adapter reference.
//!
//! One grammar serves the `specify init <adapter>` argument, recorded
//! `project.yaml.adapter` values, slice `metadata.yaml` targets, and
//! the debug resolve verbs: package references
//! (`specify:<name>@<semver>`), first-party shorthand (`omnia`,
//! `omnia@1.0.0`), and local component paths (`./adapter.wasm`,
//! `file://…/adapter.wasm`). GitHub URLs are refused.
//!
//! Parsing is purely syntactic: local-file existence,
//! canonicalization, and component validation are ensure concerns
//! (`crate::adapter::Resolver::ensure_source` / `ensure_target`), so a
//! persisted local selector still parses — and can still resolve
//! through its project cache — after the operator's original input
//! file is removed.

use std::path::{Path, PathBuf};

use error::Error;

/// The operator-supplied adapter reference, preserving its input kind.
///
/// - [`Self::Bare`] — development shorthand; resolution defers to the
///   deployment (project component cache / dev build for components,
///   catalog match for a native host).
/// - [`Self::Package`] — an immutable registry locator with a
///   mandatory exact-SemVer pin. `namespace` is parse/display
///   provenance, not a second workflow identity axis.
/// - [`Self::Component`] — a local `.wasm` component file. The kind is
///   retained so a native host can refuse it instead of silently
///   narrowing to a same-named compiled adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterSelector {
    /// Bare development shorthand (`omnia`).
    Bare {
        /// Kebab-case adapter name.
        name: String,
    },
    /// Exact package reference (`specify:omnia@1.0.0`; `omnia@1.0.0`
    /// is sugar for the `specify` namespace).
    Package {
        /// Registry namespace (`specify` for first-party adapters).
        namespace: String,
        /// Kebab-case adapter name.
        name: String,
        /// Mandatory exact SemVer pin.
        version: semver::Version,
    },
    /// Local component file path (recorded as `file://…`).
    Component {
        /// The path as supplied (relative paths are anchored at the
        /// project directory by ensure).
        path: PathBuf,
    },
}

/// The wasm-pkg namespace first-party adapters publish under
/// (`specify:<name>@<semver>` via `wkg publish` in the adapters repo;
/// specify: naming cut — `augentic:` is reserved, not routed).
pub const FIRST_PARTY_NAMESPACE: &str = "specify";

impl AdapterSelector {
    /// Parse an `<adapter>` argument or recorded adapter value.
    ///
    /// Syntactic only — never touches the filesystem.
    ///
    /// # Errors
    ///
    /// - `adapter-arg-malformed` for an empty or untrimmed value.
    /// - `adapter-github-uri-unsupported` for GitHub URLs.
    /// - `adapter-package-ref-version-required` /
    ///   `adapter-package-ref-malformed` for a package-reference shape
    ///   without an exact SemVer pin or name.
    pub fn parse(value: &str) -> Result<Self, Error> {
        if value.trim().is_empty() || value != value.trim() {
            return Err(Error::Diag {
                code: "adapter-arg-malformed",
                detail:
                    "<adapter> must be non-empty and must not have leading or trailing whitespace"
                        .to_string(),
            });
        }
        if is_github_url(value) {
            return Err(Error::Diag {
                code: "adapter-github-uri-unsupported",
                detail: format!(
                    "GitHub adapter URIs are not supported (`{value}`): a source checkout \
                     does not yield a usable adapter artifact. Pin a published component \
                     (`specify:<name>@<semver>`), point at a local `.wasm` component file, or \
                     build the in-repo development artifact (`cargo build --release --target \
                     wasm32-wasip2`)"
                ),
            });
        }
        if let Some(package) = recognize_package(value) {
            return package;
        }
        if let Some((name, version)) = parse_shorthand(value) {
            return Ok(version.map_or_else(
                || Self::Bare {
                    name: name.to_string(),
                },
                |version| Self::Package {
                    namespace: FIRST_PARTY_NAMESPACE.to_string(),
                    name: name.to_string(),
                    version,
                },
            ));
        }
        let path = value.strip_prefix("file://").unwrap_or(value);
        Ok(Self::Component {
            path: PathBuf::from(path),
        })
    }

    /// The kebab-case adapter name this selector refers to.
    ///
    /// # Errors
    ///
    /// `adapter-dir-name-unresolved` when a component path carries no
    /// usable filename stem.
    pub fn name(&self) -> Result<String, Error> {
        match self {
            Self::Bare { name } | Self::Package { name, .. } => Ok(name.clone()),
            Self::Component { path } => name_from_component(path),
        }
    }

    /// Best-effort adapter name from a recorded adapter value
    /// (`omnia@1.0.0` → `omnia`, `specify:omnia@1.0.0` → `omnia`,
    /// `file://…/specify_omnia.wasm` → `omnia`). Never fails over a
    /// malformed historical value — it echoes the raw value instead,
    /// so orchestration routing and display stay total.
    #[must_use]
    pub fn recorded_name(value: &str) -> String {
        Self::parse(value)
            .ok()
            .and_then(|selector| selector.name().ok())
            .unwrap_or_else(|| value.to_string())
    }

    /// The exact SemVer pin, present only on a package reference.
    #[must_use]
    pub const fn version(&self) -> Option<&semver::Version> {
        match self {
            Self::Package { version, .. } => Some(version),
            Self::Bare { .. } | Self::Component { .. } => None,
        }
    }

    /// The canonical wire form: the bare name, the
    /// `<namespace>:<name>@<version>` package reference, or the
    /// `file://<path>` component URI.
    #[must_use]
    pub fn wire_value(&self) -> String {
        match self {
            Self::Bare { name } => name.clone(),
            Self::Package {
                namespace,
                name,
                version,
            } => format!("{namespace}:{name}@{version}"),
            Self::Component { path } => format!("file://{}", path.display()),
        }
    }

    /// The value init records on `project.yaml.adapter`: the wire form,
    /// with a component path canonicalized (anchored at `project_dir`
    /// when relative) so the recorded selector outlives the operator's
    /// working directory. The selector kind is never rewritten.
    ///
    /// # Errors
    ///
    /// `adapter-canonicalize-failed` when the component path cannot be
    /// canonicalized (ensure has normally validated it already).
    pub fn persist_value(&self, project_dir: &Path) -> Result<String, Error> {
        match self {
            Self::Bare { .. } | Self::Package { .. } => Ok(self.wire_value()),
            Self::Component { path } => {
                let canonical = canonicalize_component(path, project_dir)?;
                Ok(format!("file://{}", canonical.display()))
            }
        }
    }
}

impl std::fmt::Display for AdapterSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.wire_value())
    }
}

/// Anchor a component path at `project_dir` when relative, then
/// canonicalize it.
///
/// # Errors
///
/// `adapter-canonicalize-failed` when the path cannot be canonicalized
/// (e.g. the file does not exist).
pub fn canonicalize_component(path: &Path, project_dir: &Path) -> Result<PathBuf, Error> {
    let absolute = if path.is_absolute() { path.to_path_buf() } else { project_dir.join(path) };
    std::fs::canonicalize(&absolute).map_err(|err| Error::Diag {
        code: "adapter-canonicalize-failed",
        detail: format!(
            "failed to canonicalize local adapter `{}` at {}: {err}",
            path.display(),
            absolute.display()
        ),
    })
}

fn is_github_url(value: &str) -> bool {
    value.starts_with("https://github.com/")
}

/// Recognise an adapter package reference `<namespace>:<name>@<semver>`.
///
/// Returns `None` when `value` is not a package-ref shape — so URL
/// schemes (`https://`, `file://`), Windows drive paths (`C:\…`), bare
/// names, and local paths keep flowing through the shorthand / local
/// branches. Returns `Some(Err(_))` when the shape *is* a package
/// reference but the version pin is missing or not exact SemVer.
fn recognize_package(value: &str) -> Option<Result<AdapterSelector, Error>> {
    let (namespace, rest) = value.split_once(':')?;
    // `//` after the colon is a URL authority (`https://`, `file://`);
    // a non-kebab namespace (e.g. the `C` of `C:\`) is a drive path.
    // Neither is a package reference.
    if rest.starts_with('/') || !is_first_party_name(namespace) {
        return None;
    }
    Some(parse_validated_package(namespace, rest, value))
}

fn parse_validated_package(
    namespace: &str, rest: &str, original: &str,
) -> Result<AdapterSelector, Error> {
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
    Ok(AdapterSelector::Package {
        namespace: namespace.to_string(),
        name: name.to_string(),
        version,
    })
}

/// Recognise a first-party adapter shorthand and split it into
/// `(name, version)`. A bare `name` carries no pin (`None`) and
/// resolves the development artifact; a `name@<semver>` carries the
/// parsed [`semver::Version`] and is sugar for the
/// `specify:<name>@<semver>` package reference. Returns `None` for
/// paths (`./foo`, `/abs`, `file://…`) and URLs (anything carrying `:`
/// or `/`), and for a `@suffix` that is not exact semver — so those
/// keep flowing through the component branch.
fn parse_shorthand(value: &str) -> Option<(&str, Option<semver::Version>)> {
    if value.contains('/') || value.contains(':') {
        return None;
    }
    match value.split_once('@') {
        Some((name, reference)) if is_first_party_name(name) => {
            let version = semver::Version::parse(reference).ok()?;
            Some((name, Some(version)))
        }
        None if is_first_party_name(value) => Some((value, None)),
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

/// Derive the kebab-case adapter name from a component filename:
/// `specify_intent.wasm` → `intent`, `my-adapter.wasm` → `my-adapter`
/// (the cargo `specify_` artifact prefix is stripped and underscores
/// fold to kebab dashes).
///
/// # Errors
///
/// `adapter-dir-name-unresolved` when the path carries no usable stem.
pub fn name_from_component(path: &Path) -> Result<String, Error> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| Error::Diag {
        code: "adapter-dir-name-unresolved",
        detail: format!("cannot derive adapter name from {}", path.display()),
    })?;
    let stem =
        stem.strip_prefix("specify_").or_else(|| stem.strip_prefix("specify-")).unwrap_or(stem);
    Ok(stem.replace('_', "-"))
}
