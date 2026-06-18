//! Structural validation for declared Specify WASI tools.

use std::collections::HashSet;
use std::path::Path;

use specify_diagnostics::{Artifact, Diagnostic};
/// Canonical JSON Schema for the two `tools:` declaration sites,
/// re-exported from the central [`specify_schema`] embed.
pub use specify_schema::EXTENSION_JSON_SCHEMA;

use crate::{
    Extension, ExtensionManifest, ExtensionScope, ExtensionSource, looks_like_sha256_hex,
    looks_like_windows_absolute,
};

const RULE_NAME_FORMAT: &str = "tool.name-format";
const RULE_VERSION_SEMVER: &str = "tool.version-is-semver";
const RULE_SOURCE_SUPPORTED: &str = "tool.source-is-supported-uri";
const RULE_PACKAGE_FORMAT: &str = "tool.package-request-format";
const RULE_PACKAGE_NAMESPACE: &str = "tool.package-namespace-first-party";
const RULE_PACKAGE_VERSION: &str = "tool.package-version-is-exact-semver";
const RULE_SHA256_FORMAT: &str = "tool.sha256-format";
const RULE_PERMISSION_PATH_FORM: &str = "tool.permission-path-form";
const RULE_LIFECYCLE_WRITE_DENIED: &str = "tool.lifecycle-state-write-denied";
const RULE_CAPABILITY_DIR_SCOPE: &str = "tool.capability-dir-out-of-scope";
const RULE_SOURCE_CAPABILITY_DIR_SCOPE: &str = "tool.source-capability-dir-out-of-scope";
const RULE_NAME_UNIQUE: &str = "tool.name-unique";

impl Extension {
    /// Validate the tool declaration against the structural rules
    /// (`name`, `version`, `source`, `sha256`, permission shape).
    /// Returns one deterministic `violation` [`Diagnostic`] per failing
    /// rule; passing rules emit nothing, so an empty vector means the
    /// tool is structurally valid.
    #[must_use]
    pub fn validate_structure(&self, scope: &ExtensionScope) -> Vec<Diagnostic> {
        let package = if let ExtensionSource::Package(p) = &self.source { Some(p) } else { None };

        let name_valid = !self.name.is_empty()
            && self.name.len() <= 64
            && self.name.as_bytes()[0].is_ascii_lowercase()
            && self.name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        let name_detail =
            (!name_valid).then(|| format!("`{}` is not a valid tool name", self.name));

        let version_detail = semver::Version::parse(&self.version)
            .err()
            .map(|err| format!("`{}` is not an exact SemVer version: {err}", self.version));

        let (source_detail, source_scope_detail) = validate_source(&self.source, scope);

        let package_format_detail = package.and_then(|p| {
            let valid = !p.namespace.is_empty()
                && !p.name.is_empty()
                && !p.version.is_empty()
                && p.to_wire_string().contains('@')
                && p.name_ref().contains(':');
            (!valid).then(|| format!("`{}` is not a package request", self.source.to_wire_string()))
        });

        let package_namespace_detail = package.and_then(|p| {
            (p.namespace != "specify")
                .then(|| format!("`{}` is not in the specify namespace", p.name_ref()))
        });

        let package_version_detail = package.and_then(|p| {
            if p.version.starts_with('v') {
                Some(format!("`{}` uses a leading v", p.version))
            } else {
                semver::Version::parse(&p.version)
                    .err()
                    .map(|err| format!("`{}` is not an exact SemVer version: {err}", p.version))
            }
        });

        let sha256_detail = self
            .sha256
            .as_deref()
            .filter(|v| !looks_like_sha256_hex(v))
            .map(|_| "`sha256` must be exactly 64 lowercase hex characters".to_string());

        [
            check(
                RULE_NAME_FORMAT,
                "tool names are lowercase kebab-case and at most 64 characters",
                name_detail,
            ),
            check(RULE_VERSION_SEMVER, "tool versions are exact SemVer versions", version_detail),
            check(
                RULE_SOURCE_SUPPORTED,
                "tool sources are absolute paths, file:// URIs, https:// URIs, $PROJECT_DIR/$CAPABILITY_DIR templates, or wasm package requests",
                source_detail,
            ),
            check(
                RULE_SOURCE_CAPABILITY_DIR_SCOPE,
                "$CAPABILITY_DIR in source is only available to plugin-scope tools",
                source_scope_detail,
            ),
            check(
                RULE_PACKAGE_FORMAT,
                "package sources use namespace:name@version syntax",
                package_format_detail,
            ),
            check(
                RULE_PACKAGE_NAMESPACE,
                "package sources use the first-party specify namespace",
                package_namespace_detail,
            ),
            check(
                RULE_PACKAGE_VERSION,
                "package sources include an exact SemVer version without a leading v",
                package_version_detail,
            ),
            check(
                RULE_SHA256_FORMAT,
                "optional sha256 pins are 64 lowercase hexadecimal characters",
                sha256_detail,
            ),
            validate_permission_paths(&self.permissions.read, &self.permissions.write),
            validate_lifecycle_writes(&self.permissions.write),
            validate_capability_dir_scope(scope, &self.permissions.read, &self.permissions.write),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl ExtensionManifest {
    /// Validate a manifest and all of its contained tools. Returns one
    /// deterministic `violation` [`Diagnostic`] per failing rule; an
    /// empty vector means the manifest is structurally valid.
    #[must_use]
    pub fn validate_structure(&self, scope: &ExtensionScope) -> Vec<Diagnostic> {
        let mut results = Vec::with_capacity(1 + self.tools.len() * 12);

        let mut seen: HashSet<&str> = HashSet::new();
        let mut duplicates: Vec<&str> = Vec::new();
        for tool in &self.tools {
            if !seen.insert(tool.name.as_str()) && !duplicates.contains(&tool.name.as_str()) {
                duplicates.push(tool.name.as_str());
            }
        }
        if let Some(diagnostic) = check(
            RULE_NAME_UNIQUE,
            "tool names are unique within a single declaration site",
            (!duplicates.is_empty())
                .then(|| format!("duplicate tool name(s): {}", duplicates.join(", "))),
        ) {
            results.push(diagnostic);
        }

        for tool in &self.tools {
            results.extend(tool.validate_structure(scope));
        }
        results
    }
}

/// Build a deterministic `violation` diagnostic for a failing rule, or
/// `None` when `detail` is absent (the rule passed).
fn check(rule_id: &'static str, rule: &'static str, detail: Option<String>) -> Option<Diagnostic> {
    detail.map(|detail| Diagnostic::violation(rule_id, rule, detail, Artifact::Plan, None))
}

fn validate_source(
    source: &ExtensionSource, scope: &ExtensionScope,
) -> (Option<String>, Option<String>) {
    let valid = match source {
        ExtensionSource::LocalPath(path) => {
            path.is_absolute() || path.to_str().is_some_and(looks_like_windows_absolute)
        }
        ExtensionSource::FileUri(uri) => {
            uri.strip_prefix("file://").is_some_and(|rest| !rest.is_empty())
        }
        ExtensionSource::HttpsUri(uri) => {
            uri.strip_prefix("https://").is_some_and(|rest| !rest.is_empty())
        }
        ExtensionSource::Package(p) => !p.name_ref().is_empty(),
        ExtensionSource::TemplatePath(t) => is_project_dir_path(t) || is_capability_dir_path(t),
    };
    let detail =
        (!valid).then(|| format!("`{}` is not a supported source", source.to_wire_string()));
    let scope_detail = if let ExtensionSource::TemplatePath(t) = source {
        (t.contains("$CAPABILITY_DIR") && !matches!(scope, ExtensionScope::Plugin { .. }))
            .then(|| "project-scope source references $CAPABILITY_DIR".to_string())
    } else {
        None
    };
    (detail, scope_detail)
}

fn validate_permission_paths(read: &[String], write: &[String]) -> Option<Diagnostic> {
    const RULE: &str = "permission paths are absolute or start with $PROJECT_DIR/$CAPABILITY_DIR, with no glob or parent segments";
    let failures: Vec<String> = read
        .iter()
        .map(|entry| ("read", entry))
        .chain(write.iter().map(|entry| ("write", entry)))
        .filter_map(|(kind, entry)| {
            permission_path_form_error(entry).map(|err| format!("{kind}: {err}"))
        })
        .collect();
    check(RULE_PERMISSION_PATH_FORM, RULE, (!failures.is_empty()).then(|| failures.join("; ")))
}

fn validate_lifecycle_writes(write: &[String]) -> Option<Diagnostic> {
    const RULE: &str = "tool write permissions do not target Specify lifecycle state";
    let failures: Vec<String> = write
        .iter()
        .filter(|entry| targets_lifecycle_state(entry))
        .map(|entry| format!("write path `{entry}` targets `.specify` lifecycle state"))
        .collect();
    check(RULE_LIFECYCLE_WRITE_DENIED, RULE, (!failures.is_empty()).then(|| failures.join("; ")))
}

fn validate_capability_dir_scope(
    scope: &ExtensionScope, read: &[String], write: &[String],
) -> Option<Diagnostic> {
    const RULE: &str = "$CAPABILITY_DIR is only available to plugin-scope tools";
    let failures: Vec<String> = if matches!(scope, ExtensionScope::Plugin { .. }) {
        Vec::new()
    } else {
        read.iter()
            .chain(write)
            .filter(|entry| entry.contains("$CAPABILITY_DIR"))
            .map(|entry| format!("project-scope permission `{entry}` references $CAPABILITY_DIR"))
            .collect()
    };
    check(RULE_CAPABILITY_DIR_SCOPE, RULE, (!failures.is_empty()).then(|| failures.join("; ")))
}

fn permission_path_form_error(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("permission path must not be empty".to_string());
    }
    if has_glob_char(value) {
        return Some(format!("`{value}` contains glob metacharacters"));
    }
    if has_parent_segment(value) {
        return Some(format!("`{value}` contains a `..` segment"));
    }
    if has_unsupported_variable(value) {
        return Some(format!("`{value}` contains an unsupported variable"));
    }
    if is_project_dir_path(value)
        || is_capability_dir_path(value)
        || Path::new(value).is_absolute()
        || looks_like_windows_absolute(value)
    {
        return None;
    }
    Some(format!("`{value}` must be absolute or start with $PROJECT_DIR or $CAPABILITY_DIR"))
}

fn has_glob_char(value: &str) -> bool {
    value.bytes().any(|b| matches!(b, b'*' | b'?' | b'[' | b']' | b'{' | b'}'))
}

fn has_parent_segment(value: &str) -> bool {
    value.split(['/', '\\']).any(|segment| segment == "..")
}

fn has_unsupported_variable(value: &str) -> bool {
    value.contains('$') && !is_project_dir_path(value) && !is_capability_dir_path(value)
}

fn is_project_dir_path(value: &str) -> bool {
    value == "$PROJECT_DIR"
        || value.starts_with("$PROJECT_DIR/")
        || value.starts_with("$PROJECT_DIR\\")
}

fn is_capability_dir_path(value: &str) -> bool {
    value == "$CAPABILITY_DIR"
        || value.starts_with("$CAPABILITY_DIR/")
        || value.starts_with("$CAPABILITY_DIR\\")
}

fn targets_lifecycle_state(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    let Some(specify_index) = normalized.find(".specify") else {
        return false;
    };
    let after = &normalized[specify_index..];
    after == ".specify" || after.starts_with(".specify/")
}

#[cfg(test)]
mod tests;
