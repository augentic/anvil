//! Manifest data types and structural validation for declared Specify
//! WASI tools.
//!
//! A wasmtime-free leaf: `specify-workflow` consumes the DTOs (the
//! `tools:` field on `project.yaml` and the init-time wasm-pkg config
//! constants) without linking the Wasmtime execution host, which stays
//! in `specify-tool` alongside the cache, resolver, and runner.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod validate;

/// Filename of the project-local wasm-pkg config inside `.specify/`.
///
/// Paired with `Layout::specify_dir` (specify-workflow) at the init
/// site so the helper does not have to re-derive the relative path.
pub const WASM_PKG_CONFIG_FILENAME: &str = "wasm-pkg.toml";

/// Project-rooted relative path to the project-local wasm-pkg config.
///
/// Merged in between the global wasm-pkg defaults and the `WKG_CONFIG`
/// override. Operators edit this file to add namespace mappings
/// (private mirrors, internal registries) without setting an env var.
pub const WASM_PKG_CONFIG_PATH: &str = ".specify/wasm-pkg.toml";

/// Canonical contents `specify init` writes for a fresh project.
///
/// Mirrors the wasm-pkg distribution model so
/// `wkg --config .specify/wasm-pkg.toml` and `specify tool fetch`
/// agree on namespace routing.
pub const DEFAULT_WASM_PKG_CONFIG: &str = "default_registry = \"augentic.io\"\n\
                                           \n\
                                           [namespace_registries]\n\
                                           specify = \"augentic.io\"\n";

/// One declared WASI tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ToolObject")]
pub struct Extension {
    /// Extension name used by `specify tool run <name>`.
    pub name: String,
    /// Exact SemVer version string. Parsed during structural validation.
    pub version: String,
    /// Source of the WASI component bytes.
    pub source: ExtensionSource,
    /// Optional lower-case hex SHA-256 digest over the component bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Filesystem preopen requests.
    #[serde(default, skip_serializing_if = "ExtensionPermissions::is_default")]
    pub permissions: ExtensionPermissions,
}

/// Supported source locations for WASI component bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum ExtensionSource {
    /// Absolute local filesystem path.
    LocalPath(PathBuf),
    /// `file://` URI.
    FileUri(String),
    /// `https://` URI.
    HttpsUri(String),
    /// Exact wasm-pkg package request.
    Package(PackageRequest),
    /// Template path starting with `$PROJECT_DIR` or `$CAPABILITY_DIR`.
    /// Expanded to a [`LocalPath`](Self::LocalPath) at resolution time
    /// when the project directory is known.
    TemplatePath(String),
}

/// Exact wasm-pkg package request used by first-party tool declarations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRequest {
    /// Package namespace before `:`.
    pub namespace: String,
    /// Package name after `:` and before `@`.
    pub name: String,
    /// Exact version after `@`.
    pub version: String,
}

impl ExtensionSource {
    /// Classify a manifest `source:` string into a supported source variant.
    ///
    /// # Errors
    ///
    /// Returns an error message when the source uses a relative path or an
    /// unsupported URI scheme.
    pub fn parse_wire(value: &str) -> Result<Self, String> {
        if value.starts_with("https://") {
            Ok(Self::HttpsUri(value.to_string()))
        } else if value.starts_with("file://") {
            Ok(Self::FileUri(value.to_string()))
        } else if Path::new(value).is_absolute() || looks_like_windows_absolute(value) {
            Ok(Self::LocalPath(PathBuf::from(value)))
        } else if looks_like_template_path(value) {
            Ok(Self::TemplatePath(value.to_string()))
        } else if looks_like_package_request(value) {
            Ok(Self::Package(PackageRequest::parse(value)))
        } else {
            Err(format!(
                "unsupported tool source `{value}`; expected an absolute path, file:// URI, https:// URI, $PROJECT_DIR/$CAPABILITY_DIR template, or wasm package request"
            ))
        }
    }

    /// Return the manifest string form for this source.
    #[must_use]
    pub fn to_wire_string(&self) -> Cow<'_, str> {
        match self {
            Self::LocalPath(path) => path.to_string_lossy(),
            Self::FileUri(uri) | Self::HttpsUri(uri) => Cow::Borrowed(uri),
            Self::Package(package) => Cow::Owned(package.to_wire_string()),
            Self::TemplatePath(template) => Cow::Borrowed(template),
        }
    }

    /// Expand a [`TemplatePath`](Self::TemplatePath) into a [`LocalPath`](Self::LocalPath)
    /// by substituting `$PROJECT_DIR` and `$CAPABILITY_DIR`.
    ///
    /// Non-template variants are returned unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error when the template references `$CAPABILITY_DIR` but
    /// no plugin capability directory is provided, when the expanded path is not
    /// absolute, or when the root directory is not valid UTF-8.
    pub fn expand(
        &self, project_dir: &Path, capability_dir: Option<&Path>,
    ) -> Result<Self, String> {
        let Self::TemplatePath(template) = self else {
            return Ok(self.clone());
        };
        let project = project_dir.to_str().ok_or("$PROJECT_DIR contains non-UTF-8 bytes")?;
        let mut expanded = template.replace("$PROJECT_DIR", project);
        if expanded.contains("$CAPABILITY_DIR") {
            let adapter = capability_dir
                .ok_or("$CAPABILITY_DIR is only available to plugin-scope tools")?
                .to_str()
                .ok_or("$CAPABILITY_DIR contains non-UTF-8 bytes")?;
            expanded = expanded.replace("$CAPABILITY_DIR", adapter);
        }
        let path = PathBuf::from(&expanded);
        if !path.is_absolute() {
            return Err(format!("expanded source path must be absolute: {expanded}"));
        }
        Ok(Self::LocalPath(path))
    }
}

impl PackageRequest {
    /// Parse a package request string.
    ///
    /// Parsing is intentionally permissive so structural validation can emit
    /// stable rule ids for unsupported namespaces and non-SemVer versions.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        let (package, version) = value.split_once('@').unwrap_or((value, ""));
        let (namespace, name) = package.split_once(':').unwrap_or(("", package));
        Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    /// Return the package name without the version suffix.
    #[must_use]
    pub fn name_ref(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }

    /// Return the manifest string form.
    #[must_use]
    pub fn to_wire_string(&self) -> String {
        format!("{}@{}", self.name_ref(), self.version)
    }
}

impl From<ExtensionSource> for String {
    fn from(value: ExtensionSource) -> Self {
        value.to_wire_string().into_owned()
    }
}

impl TryFrom<String> for ExtensionSource {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse_wire(&value)
    }
}

/// The full object form of a declared tool. The scalar first-party
/// shorthand (`specify:<name>@<ver>`) and its embedded permissions
/// catalog are retired (RFC-48 D10): every declaration spells out its
/// own `source` and `permissions`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolObject {
    name: String,
    version: String,
    source: ExtensionSource,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    permissions: ExtensionPermissions,
}

impl From<ToolObject> for Extension {
    fn from(object: ToolObject) -> Self {
        let ToolObject {
            name,
            version,
            source,
            sha256,
            permissions,
        } = object;
        Self {
            name,
            version,
            source,
            sha256,
            permissions,
        }
    }
}

/// Filesystem permissions requested by a tool.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPermissions {
    /// Read-only preopen path templates.
    #[serde(default)]
    pub read: Vec<String>,
    /// Read-write preopen path templates.
    #[serde(default)]
    pub write: Vec<String>,
}

impl ExtensionPermissions {
    /// True when no read or write preopen paths are requested — the
    /// serde `skip_serializing_if` predicate for an omitted permissions
    /// block.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        self.read.is_empty() && self.write.is_empty()
    }
}

/// A `tools:` array as it appears in either declaration site.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    /// Declared tools.
    #[serde(default)]
    pub tools: Vec<Extension>,
}

/// Plugin axis discriminator per workflow §Adapter vocabulary.
///
/// Source plugins (`extract` / `survey`) and target plugins
/// (`shape` / `build` / `merge`) share the `adapter.yaml` shape and
/// on-disk filename; `Axis` is what disambiguates them in
/// [`ExtensionScope::Plugin`] and in the out-of-tree cache layout under
/// `<project-cache>/manifests/{sources,targets}/<name>/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Axis {
    /// Source plugin.
    Source,
    /// Target plugin.
    Target,
}

impl Axis {
    /// Directory segment under `<project_dir>/` and the out-of-tree
    /// `<project-cache>/manifests/` — `"sources"` for source plugins,
    /// `"targets"` for target plugins.
    #[must_use]
    pub const fn dir_segment(self) -> &'static str {
        match self {
            Self::Source => "sources",
            Self::Target => "targets",
        }
    }
}

/// Identifies which declaration site a tool came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::EnumDiscriminants)]
#[strum_discriminants(name(ExtensionScopeKind))]
#[strum_discriminants(derive(Hash, serde::Serialize, serde::Deserialize, strum::Display))]
#[strum_discriminants(serde(rename_all = "kebab-case"))]
#[strum_discriminants(strum(serialize_all = "kebab-case"))]
pub enum ExtensionScope {
    /// Extension declared in `.specify/project.yaml`.
    Project {
        /// Project name from `project.yaml`.
        project_name: String,
    },
    /// Extension declared in a resolved plugin's sidecar `tools.yaml`.
    /// Per workflow §Adapter implementation shape, plugins carry an
    /// [`Axis`] (`source` / `target`); the read-only plugin-owned
    /// cache root exposed to guests as `$CAPABILITY_DIR` is
    /// `capability_dir`.
    Plugin {
        /// Axis discriminator from the plugin manifest.
        axis: Axis,
        /// Plugin slug from `adapter.yaml`.
        plugin_slug: String,
        /// Resolved plugin directory used as `$CAPABILITY_DIR`.
        capability_dir: PathBuf,
    },
}

/// True when `value` has the `<drive>:<separator>` shape of a Windows
/// absolute path (e.g. `C:\tools` or `C:/tools`), which
/// `Path::is_absolute` misses on non-Windows hosts.
#[must_use]
pub fn looks_like_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

/// True when `value` is a 64-character lowercase hexadecimal SHA-256 digest.
#[must_use]
pub fn looks_like_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn looks_like_package_request(value: &str) -> bool {
    value.contains(':')
}

fn looks_like_template_path(value: &str) -> bool {
    is_template_var_prefix(value, "$PROJECT_DIR")
        || is_template_var_prefix(value, "$CAPABILITY_DIR")
}

fn is_template_var_prefix(value: &str, var: &str) -> bool {
    value == var || value.starts_with(&format!("{var}/")) || value.starts_with(&format!("{var}\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_all_sources() {
        let manifest = ExtensionManifest {
            tools: vec![
                Extension {
                    name: "local-tool".to_string(),
                    version: "1.0.0".to_string(),
                    source: ExtensionSource::LocalPath(PathBuf::from("/opt/specify/local.wasm")),
                    sha256: None,
                    permissions: ExtensionPermissions::default(),
                },
                Extension {
                    name: "file-tool".to_string(),
                    version: "1.0.1".to_string(),
                    source: ExtensionSource::FileUri("file:///opt/specify/file.wasm".to_string()),
                    sha256: None,
                    permissions: ExtensionPermissions {
                        read: vec!["$PROJECT_DIR/contracts".to_string()],
                        write: Vec::new(),
                    },
                },
                Extension {
                    name: "https-tool".to_string(),
                    version: "1.0.2".to_string(),
                    source: ExtensionSource::HttpsUri(
                        "https://example.com/specify/https.wasm".to_string(),
                    ),
                    sha256: Some(
                        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                            .to_string(),
                    ),
                    permissions: ExtensionPermissions {
                        read: Vec::new(),
                        write: vec!["$PROJECT_DIR/generated".to_string()],
                    },
                },
            ],
        };

        let yaml = serde_saphyr::to_string(&manifest).expect("serialize manifest");
        assert!(yaml.contains("source: /opt/specify/local.wasm"));
        assert!(yaml.contains("source: file:///opt/specify/file.wasm"));
        assert!(yaml.contains("source: https://example.com/specify/https.wasm"));

        let parsed: ExtensionManifest = serde_saphyr::from_str(&yaml).expect("parse manifest");
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn unsupported_source_fails() {
        serde_saphyr::from_str::<ExtensionManifest>(
            "tools:\n  - name: bad\n    version: 1.0.0\n    source: relative.wasm\n",
        )
        .expect_err("relative source must fail");
    }

    #[test]
    fn scalar_package_form_rejected() {
        // The first-party scalar shorthand and its embedded permissions
        // catalog are retired (RFC-48 D10): a tool is always a full
        // object spelling out its own `source` and `permissions`. A bare
        // package string no longer deserializes.
        serde_saphyr::from_str::<ExtensionManifest>("tools:\n  - \"specify:contract@1.2.3\"\n")
            .expect_err("scalar first-party form must be rejected");
        serde_saphyr::from_str::<ExtensionManifest>("tools:\n  - \"other:helper@latest\"\n")
            .expect_err("scalar package form must be rejected");
    }

    #[test]
    fn object_package_source_round_trips() {
        // A package `source:` string inside the object form is still a
        // valid `ExtensionSource::Package` — only the top-level scalar
        // entry shorthand is gone.
        let manifest: ExtensionManifest = serde_saphyr::from_str(
            "tools:\n  - name: contract\n    version: 1.2.3\n    source: \"specify:contract@1.2.3\"\n",
        )
        .expect("parse object package manifest");
        let tool = &manifest.tools[0];
        assert_eq!(tool.name, "contract");
        assert!(matches!(
            &tool.source,
            ExtensionSource::Package(package)
                if package.namespace == "specify"
                    && package.name == "contract"
                    && package.version == "1.2.3"
        ));
        assert_eq!(tool.permissions, ExtensionPermissions::default());
    }

    #[test]
    fn template_source_round_trips() {
        let manifest: ExtensionManifest = serde_saphyr::from_str(
            "tools:\n  - name: vectis\n    version: 0.3.0\n    source: $PROJECT_DIR/../specify-cli/target/vectis.wasm\n",
        )
        .expect("parse template source");
        let tool = &manifest.tools[0];
        assert!(
            matches!(&tool.source, ExtensionSource::TemplatePath(t) if t == "$PROJECT_DIR/../specify-cli/target/vectis.wasm"),
        );
        let yaml = serde_saphyr::to_string(&manifest).expect("serialize template source");
        assert!(yaml.contains("source: $PROJECT_DIR/../specify-cli/target/vectis.wasm"), "{yaml}");
    }

    #[test]
    fn expand_replaces_project_dir() {
        let source = ExtensionSource::TemplatePath("$PROJECT_DIR/tools/vectis.wasm".to_string());
        let expanded = source.expand(Path::new("/home/user/project"), None).expect("expand");
        assert_eq!(
            expanded,
            ExtensionSource::LocalPath(PathBuf::from("/home/user/project/tools/vectis.wasm"))
        );
    }

    #[test]
    fn expand_replaces_capability_dir() {
        let source = ExtensionSource::TemplatePath("$CAPABILITY_DIR/bin/tool.wasm".to_string());
        let expanded =
            source.expand(Path::new("/project"), Some(Path::new("/caps/vectis"))).expect("expand");
        assert_eq!(
            expanded,
            ExtensionSource::LocalPath(PathBuf::from("/caps/vectis/bin/tool.wasm"))
        );
    }

    #[test]
    fn expand_rejects_capability_dir() {
        let source = ExtensionSource::TemplatePath("$CAPABILITY_DIR/bin/tool.wasm".to_string());
        source.expand(Path::new("/project"), None).expect_err("must reject missing adapter dir");
    }

    #[test]
    fn expand_identity_for_non_template() {
        let source = ExtensionSource::LocalPath(PathBuf::from("/absolute/path.wasm"));
        let expanded = source.expand(Path::new("/project"), None).expect("expand");
        assert_eq!(expanded, ExtensionSource::LocalPath(PathBuf::from("/absolute/path.wasm")));
    }

    #[test]
    fn template_requires_boundary() {
        assert!(!looks_like_template_path("$PROJECT_DIRX/foo.wasm"));
        assert!(!looks_like_template_path("$CAPABILITY_DIRX/foo.wasm"));
        assert!(looks_like_template_path("$PROJECT_DIR/foo.wasm"));
        assert!(looks_like_template_path("$CAPABILITY_DIR/foo.wasm"));
        assert!(looks_like_template_path("$PROJECT_DIR"));
        assert!(looks_like_template_path("$CAPABILITY_DIR"));
    }

    // `parse_wire` is the single classifier every wire string flows
    // through; one drift in the prefix order (e.g. classifying a
    // `$PROJECT_DIR` template as a package because it contains no `:`)
    // would silently mis-route a source. Pin each arm, including the
    // Windows-absolute branch that string-prefix checks alone miss.
    #[test]
    fn parse_wire_classifies_each_scheme() {
        assert!(matches!(
            ExtensionSource::parse_wire("https://example.com/t.wasm"),
            Ok(ExtensionSource::HttpsUri(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("file:///opt/t.wasm"),
            Ok(ExtensionSource::FileUri(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("/opt/specify/t.wasm"),
            Ok(ExtensionSource::LocalPath(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire(r"C:\tools\t.wasm"),
            Ok(ExtensionSource::LocalPath(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("C:/tools/t.wasm"),
            Ok(ExtensionSource::LocalPath(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("$PROJECT_DIR/tools/t.wasm"),
            Ok(ExtensionSource::TemplatePath(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("$CAPABILITY_DIR"),
            Ok(ExtensionSource::TemplatePath(_))
        ));
        assert!(matches!(
            ExtensionSource::parse_wire("specify:contract@1.0.0"),
            Ok(ExtensionSource::Package(_))
        ));
        ExtensionSource::parse_wire("relative/t.wasm")
            .expect_err("relative path is unclassifiable");
    }

    // `PackageRequest::parse` is deliberately permissive so structural
    // validation can emit stable rule ids; verify the split points so a
    // refactor of the `@` / `:` handling cannot quietly swap which field
    // captures a missing separator.
    #[test]
    fn package_request_parse_edges() {
        let full = PackageRequest::parse("specify:contract@1.2.3");
        assert_eq!(
            (full.namespace.as_str(), full.name.as_str(), full.version.as_str()),
            ("specify", "contract", "1.2.3")
        );

        let no_version = PackageRequest::parse("specify:contract");
        assert_eq!(
            (no_version.namespace.as_str(), no_version.name.as_str(), no_version.version.as_str()),
            ("specify", "contract", "")
        );

        let no_namespace = PackageRequest::parse("contract@1.2.3");
        assert_eq!(
            (
                no_namespace.namespace.as_str(),
                no_namespace.name.as_str(),
                no_namespace.version.as_str()
            ),
            ("", "contract", "1.2.3")
        );

        let bare = PackageRequest::parse("contract");
        assert_eq!(
            (bare.namespace.as_str(), bare.name.as_str(), bare.version.as_str()),
            ("", "contract", "")
        );

        // The version split happens before the namespace split, so a
        // second `@` stays inside the version segment.
        let extra_at = PackageRequest::parse("specify:contract@1@2");
        assert_eq!(extra_at.version, "1@2");
    }

    #[test]
    fn sha256_hex_form_is_strict() {
        let valid = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(looks_like_sha256_hex(valid));
        assert!(!looks_like_sha256_hex(&valid[..63]), "63 chars is too short");
        assert!(!looks_like_sha256_hex(&format!("{valid}0")), "65 chars is too long");
        assert!(!looks_like_sha256_hex(&valid.to_ascii_uppercase()), "uppercase hex is rejected");
        let with_g = format!("g{}", &valid[1..]);
        assert!(!looks_like_sha256_hex(&with_g), "non-hex letter rejected");
    }

    #[test]
    fn windows_absolute_requires_drive_shape() {
        assert!(looks_like_windows_absolute(r"C:\dir"));
        assert!(looks_like_windows_absolute("C:/dir"));
        assert!(!looks_like_windows_absolute("C:dir"), "drive without separator is not absolute");
        assert!(!looks_like_windows_absolute("1:/dir"), "drive letter must be alphabetic");
        assert!(!looks_like_windows_absolute("C"), "too short");
    }
}
