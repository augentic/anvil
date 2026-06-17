//! In-process `extension` framework checker (Road B `kind: tool`).
//!
//! Covers `adapter-extension-crate-missing` (CORE-061): when an
//! `adapter.yaml` declares a top-level `extension:` block, the adapter
//! must ship its WASI extension from its own tree — a co-located Rust
//! crate at `<adapter>/extension/` and the committed, built
//! `<adapter>/adapter.wasm` at the adapter root. Carries no policy —
//! the co-located-crate / committed-wasm layout is structural.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::support::{ToolFinding, relative_display};

const RULE_EXTENSION_CRATE_MISSING: &str = "CORE-061";

const IMPACT: &str = "An adapter declares a wasm extension but ships no co-located crate or committed adapter.wasm, so the extension cannot be built or run from the published adapter tree.";
const REMEDIATION: &str = "Add the co-located extension crate at <adapter>/extension/ and commit the built <adapter>/adapter.wasm (run `specify adapter build`), or remove the adapter.yaml.extension declaration.";

/// Tolerant manifest view: only the presence of the top-level
/// `extension:` block matters here; every other field is ignored, so a
/// manifest's wider shape stays the adapter-schema rule's concern.
#[derive(Debug, Deserialize)]
struct ManifestExtension {
    extension: Option<serde_json::Value>,
}

/// Run the extension-crate presence check (whole-tree; args carry no
/// policy).
pub fn run(project_dir: &Path, _args: &[String]) -> Vec<ToolFinding> {
    check_extension_crates(project_dir)
}

fn finding(rel: &str, message: String) -> ToolFinding {
    ToolFinding {
        rule_id: RULE_EXTENSION_CRATE_MISSING,
        path: Some(rel.to_string()),
        message,
        impact: IMPACT,
        remediation: REMEDIATION,
    }
}

/// CORE-061: every adapter whose `adapter.yaml` declares a top-level
/// `extension:` block must carry a co-located `extension/` crate
/// directory and a committed root `adapter.wasm`.
fn check_extension_crates(project_dir: &Path) -> Vec<ToolFinding> {
    let mut findings = Vec::new();
    for adapter_dir in adapter_dirs(project_dir) {
        let manifest_path = adapter_dir.join("adapter.yaml");
        if !declares_extension(&manifest_path) {
            continue;
        }
        let crate_missing = !adapter_dir.join("extension").is_dir();
        let wasm_missing = !adapter_dir.join("adapter.wasm").is_file();
        if !crate_missing && !wasm_missing {
            continue;
        }
        let mut missing: Vec<&str> = Vec::new();
        if crate_missing {
            missing.push("co-located crate directory `extension/`");
        }
        if wasm_missing {
            missing.push("committed `adapter.wasm`");
        }
        let name = adapter_name(&adapter_dir);
        findings.push(finding(
            &relative_display(project_dir, &manifest_path),
            format!(
                "Adapter '{name}' declares adapter.yaml.extension but is missing its {}",
                missing.join(" and ")
            ),
        ));
    }
    findings.sort_by(|a, b| a.message.cmp(&b.message));
    findings
}

/// Whether `manifest_path` parses as YAML carrying a top-level
/// `extension:` block. Unreadable or unparseable manifests are treated
/// as not declaring one — manifest-shape findings are the adapter
/// schema rule's responsibility, not this checker's.
fn declares_extension(manifest_path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(manifest_path) else {
        return false;
    };
    let Ok(body) = serde_saphyr::from_str::<ManifestExtension>(&text) else {
        return false;
    };
    body.extension.is_some()
}

/// Immediate adapter directories under both axes
/// (`adapters/{sources,targets}/<adapter>/`), skipping symlinks and
/// non-directories. Sorted for deterministic finding order.
fn adapter_dirs(project_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for axis in ["sources", "targets"] {
        let axis_dir = project_dir.join("adapters").join(axis);
        let Ok(entries) = std::fs::read_dir(&axis_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    dirs
}

/// The adapter's directory-name handle for operator-facing messages.
fn adapter_name(adapter_dir: &Path) -> String {
    adapter_dir.file_name().and_then(|name| name.to_str()).unwrap_or("<unknown>").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_WITH_EXTENSION: &str = "name: withext\nversion: \"1.0.0\"\naxis: target\nextension:\n  name: withext\n  permissions:\n    read:\n      - $PROJECT_DIR\n";
    const MANIFEST_WITHOUT_EXTENSION: &str =
        "name: plain\nversion: \"1.0.0\"\naxis: target\nbriefs:\n  shape: briefs/shape.md\n";

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    #[test]
    fn declared_and_complete_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "adapters/targets/withext/adapter.yaml", MANIFEST_WITH_EXTENSION);
        write(dir.path(), "adapters/targets/withext/extension/Cargo.toml", "[package]\n");
        write(dir.path(), "adapters/targets/withext/adapter.wasm", "wasm-bytes");
        assert!(check_extension_crates(dir.path()).is_empty());
    }

    #[test]
    fn declared_missing_crate_dir_is_flagged() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "adapters/targets/withext/adapter.yaml", MANIFEST_WITH_EXTENSION);
        write(dir.path(), "adapters/targets/withext/adapter.wasm", "wasm-bytes");
        let findings = check_extension_crates(dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_EXTENSION_CRATE_MISSING);
        assert_eq!(findings[0].path.as_deref(), Some("adapters/targets/withext/adapter.yaml"));
        assert!(findings[0].message.contains("extension/"));
    }

    #[test]
    fn declared_missing_wasm_is_flagged() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "adapters/targets/withext/adapter.yaml", MANIFEST_WITH_EXTENSION);
        write(dir.path(), "adapters/targets/withext/extension/Cargo.toml", "[package]\n");
        let findings = check_extension_crates(dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_EXTENSION_CRATE_MISSING);
        assert!(findings[0].message.contains("adapter.wasm"));
    }

    #[test]
    fn not_declared_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "adapters/targets/plain/adapter.yaml", MANIFEST_WITHOUT_EXTENSION);
        write(dir.path(), "adapters/sources/intent/adapter.yaml", MANIFEST_WITHOUT_EXTENSION);
        assert!(check_extension_crates(dir.path()).is_empty());
    }
}
