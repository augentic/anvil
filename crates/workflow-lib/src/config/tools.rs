//! The project-scope `tools[]` declaration shape on `project.yaml`.
//!
//! Serde DTOs only: no verb resolves or runs declared tools today (the
//! Wasmtime runner retired with the native provisioning surface), so
//! the shape is kept parse-clean until the `tools[]` surface's fate is
//! decided. Absorbed from the deleted `specify-extension` leaf.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One declared WASI tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ToolObject")]
pub struct Extension {
    /// Extension name.
    pub name: String,
    /// Exact SemVer version string.
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
        } else if value.contains(':') {
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
}

impl PackageRequest {
    /// Parse a package request string. Intentionally permissive so a
    /// future validation surface can emit stable rule ids for
    /// unsupported namespaces and non-SemVer versions.
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

    /// Return the manifest string form.
    #[must_use]
    pub fn to_wire_string(&self) -> String {
        format!("{}:{}@{}", self.namespace, self.name, self.version)
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

/// The full object form of a declared tool: every declaration spells
/// out its own `source` and `permissions` (there is no scalar
/// shorthand).
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

/// True when `value` has the `<drive>:<separator>` shape of a Windows
/// absolute path (e.g. `C:\tools` or `C:/tools`), which
/// `Path::is_absolute` misses on non-Windows hosts.
fn looks_like_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn looks_like_template_path(value: &str) -> bool {
    is_template_var_prefix(value, "$PROJECT_DIR")
        || is_template_var_prefix(value, "$CAPABILITY_DIR")
}

fn is_template_var_prefix(value: &str, var: &str) -> bool {
    value == var || value.starts_with(&format!("{var}/")) || value.starts_with(&format!("{var}\\"))
}
