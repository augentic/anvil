//! Keep concrete adapter crates out of the workflow engine.
//!
//! Parses every engine Cargo manifest and inspects each dependency
//! table: an entry is a violation when its effective package name is a
//! concrete adapter crate, or when its `path`/`git` source points into
//! `specify-adapters`. Rust sources need no scan — a crate that no
//! manifest declares cannot be imported.

use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

/// Engine manifest scopes: the workspace root plus every crate and
/// examples manifest.
const SCOPES: &[&str] = &["Cargo.toml", "crates", "examples"];

/// Crates owned by the `specify-adapters` repository. `omnia` is absent
/// on purpose: the name collides with the Omnia runtime crate, so the
/// omnia target adapter is caught by the repository-source rule instead.
/// `prose` is absent too: the engine owns its own `crates/prose`
/// build-time codegen crate of the same name. `adapter` is absent since
/// RFC-61: the SDK is the engine-owned `crates/adapter`; a dependency
/// reaching into the adapters repository is still caught by the
/// repository-source rule.
const ADAPTER_CRATES: &[&str] =
    &["captures", "contracts", "documentation", "intent", "screenshots", "typescript", "vectis"];

const ADAPTER_REPOSITORY: &str = "specify-adapters";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/checks/ sits two levels under the repo root")
        .to_path_buf()
}

fn findings(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for manifest in manifests(root) {
        let Ok(body) = fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(document) = body.parse::<Table>() else {
            continue;
        };
        let relative = rel(root, &manifest);
        for (table, name, detail) in violations(&document) {
            out.push(format!("{relative}: [{table}] {name}: {detail}"));
        }
    }
    out
}

fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    for scope in SCOPES {
        let path = root.join(scope);
        if path.is_file() {
            manifests.push(path);
            continue;
        }
        let mut files = Vec::new();
        walk_files(&path, &mut files);
        manifests.extend(
            files
                .into_iter()
                .filter(|file| file.file_name().is_some_and(|name| name == "Cargo.toml")),
        );
    }
    manifests
}

fn violations(document: &Table) -> Vec<(String, String, String)> {
    let mut violations = Vec::new();
    for (table, entries) in dependency_tables(document) {
        for (name, spec) in entries {
            if let Some(detail) = offence(name, spec) {
                violations.push((table.clone(), name.clone(), detail));
            }
        }
    }
    violations
}

fn dependency_tables(root: &Table) -> Vec<(String, &Table)> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut tables = Vec::new();
    for kind in KINDS {
        if let Some(entries) = root.get(kind).and_then(Value::as_table) {
            tables.push((kind.to_string(), entries));
        }
    }
    if let Some(entries) = root
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Value::as_table)
    {
        tables.push(("workspace.dependencies".to_string(), entries));
    }
    if let Some(targets) = root.get("target").and_then(Value::as_table) {
        for (cfg, entry) in targets {
            let Some(entry) = entry.as_table() else {
                continue;
            };
            for kind in KINDS {
                if let Some(entries) = entry.get(kind).and_then(Value::as_table) {
                    tables.push((format!("target.{cfg}.{kind}"), entries));
                }
            }
        }
    }
    tables
}

fn offence(name: &str, spec: &Value) -> Option<String> {
    let table = spec.as_table();
    let package =
        table.and_then(|table| table.get("package")).and_then(Value::as_str).unwrap_or(name);
    let effective = package.replace('_', "-");
    if ADAPTER_CRATES.contains(&effective.as_str()) {
        return Some(format!("depends on the concrete adapter crate `{effective}`"));
    }
    let table = table?;
    for key in ["path", "git"] {
        if let Some(source) = table.get(key).and_then(Value::as_str)
            && source.contains(ADAPTER_REPOSITORY)
        {
            return Some(format!("`{key}` points into {ADAPTER_REPOSITORY}: {source}"));
        }
    }
    None
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_files(&path, out);
        } else if file_type.is_file() {
            out.push(path);
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

#[test]
fn repo_has_no_adapter_dependencies() {
    let findings = findings(&repo_root());
    assert!(findings.is_empty(), "adapter boundary violated:\n{findings:#?}");
}

/// Workflow-core crates must not production- or build-depend on the
/// native host, its labs, or test support; `native` and `eval` must
/// not production-depend on what would drag concrete adapters, Cursor
/// integration, or the workflow surface they bypass into the host.
mod direction {
    use super::*;

    /// Crates forming the deployment-neutral workflow core, plus the
    /// Wasm deployment's `guest` crate (which must stay equally free of
    /// the native host and its labs).
    const WORKFLOW_CORE: &[&str] = &[
        "error",
        "diagnostics",
        "artifacts",
        "adapter",
        "project",
        "slice",
        "change",
        "transport",
        "guest",
    ];

    /// Dependencies the workflow core rejects outside dev-dependencies.
    const HOST_CRATES: &[&str] = &["native", "eval", "mock", "lab", "harness"];

    /// Production dependencies `native` rejects (concrete adapter
    /// crates are already caught by the repository-wide rule).
    const NATIVE_REJECTS: &[&str] = &["mock", "eval", "lab", "harness", "omnia-cursor"];

    /// Production dependencies `eval` rejects (`native` itself is its
    /// one host dependency).
    const EVAL_REJECTS: &[&str] = &["mock", "lab", "harness", "omnia-cursor", "change"];

    /// Effective package names declared under the root
    /// `[workspace.dependencies]`, keyed by alias, so a member's
    /// `host.workspace = true` resolves to the real package.
    fn workspace_aliases(root: &Path) -> std::collections::BTreeMap<String, String> {
        let mut aliases = std::collections::BTreeMap::new();
        let Some(document) = fs::read_to_string(root.join("Cargo.toml"))
            .ok()
            .and_then(|body| body.parse::<Table>().ok())
        else {
            return aliases;
        };
        let Some(entries) = document
            .get("workspace")
            .and_then(Value::as_table)
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(Value::as_table)
        else {
            return aliases;
        };
        for (name, spec) in entries {
            let package = spec
                .as_table()
                .and_then(|table| table.get("package"))
                .and_then(Value::as_str)
                .unwrap_or(name);
            aliases.insert(name.clone(), package.replace('_', "-"));
        }
        aliases
    }

    fn production_dependencies(
        document: &Table, aliases: &std::collections::BTreeMap<String, String>,
    ) -> Vec<(String, String)> {
        dependency_tables(document)
            .into_iter()
            .filter(|(table, _)| !table.contains("dev-dependencies"))
            .flat_map(|(table, entries)| {
                // Resolve the effective package name — a local
                // `package = "…"` rename first, then the root
                // workspace declaration behind `workspace = true` — so
                // a Cargo alias cannot bypass the direction rules.
                entries.iter().map(move |(name, spec)| {
                    let package = spec
                        .as_table()
                        .and_then(|table| table.get("package"))
                        .and_then(Value::as_str)
                        .map_or_else(
                            || aliases.get(name).cloned().unwrap_or_else(|| name.clone()),
                            ToString::to_string,
                        );
                    (table.clone(), package.replace('_', "-"))
                })
            })
            .collect()
    }

    fn direction_findings(root: &Path) -> Vec<String> {
        let aliases = workspace_aliases(root);
        let mut out = Vec::new();
        for (member, rejected) in std::iter::empty()
            .chain(WORKFLOW_CORE.iter().map(|name| (*name, HOST_CRATES)))
            .chain([("native", NATIVE_REJECTS), ("eval", EVAL_REJECTS)])
        {
            let manifest = root.join("crates").join(member).join("Cargo.toml");
            let Ok(body) = fs::read_to_string(&manifest) else {
                continue;
            };
            let Ok(document) = body.parse::<Table>() else {
                continue;
            };
            for (table, name) in production_dependencies(&document, &aliases) {
                if rejected.contains(&name.as_str()) {
                    out.push(format!("crates/{member}/Cargo.toml: [{table}] {name}"));
                }
            }
        }
        out
    }

    #[test]
    fn workflow_core_stays_host_free() {
        let findings = direction_findings(&repo_root());
        assert!(findings.is_empty(), "dependency direction violated:\n{findings:#?}");
    }

    #[test]
    fn bad_fixtures() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "crates/slice/Cargo.toml", "[dependencies]\nnative = \"1\"\n");
        assert!(!direction_findings(dir.path()).is_empty());

        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "crates/native/Cargo.toml", "[dependencies]\nmock = \"1\"\n");
        assert!(!direction_findings(dir.path()).is_empty());

        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "crates/eval/Cargo.toml", "[dependencies]\nomnia-cursor = \"1\"\n");
        assert!(!direction_findings(dir.path()).is_empty());

        // A Cargo alias cannot smuggle a rejected package past the rule.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/slice/Cargo.toml",
            "[dependencies]\nhost = { package = \"native\", version = \"1\" }\n",
        );
        assert!(!direction_findings(dir.path()).is_empty());

        // Nor can an alias inherited from the root workspace declaration.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.dependencies]\nhost = { package = \"native\", path = \"crates/native\" }\n",
        );
        write(
            dir.path(),
            "crates/slice/Cargo.toml",
            "[dependencies]\nhost = { workspace = true }\n",
        );
        assert!(!direction_findings(dir.path()).is_empty());

        // Dev-dependencies on host/test-support crates remain legal.
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "crates/change/Cargo.toml", "[dev-dependencies]\nmock = \"1\"\n");
        assert!(direction_findings(dir.path()).is_empty());
    }
}

#[test]
fn bad_fixtures() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "crates/slice/Cargo.toml",
        "[dependencies]\nvectis = { path = \"../../specify-adapters/targets/vectis\" }\n",
    );
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "crates/slice/Cargo.toml",
        "[dev-dependencies]\nharmless = { package = \"captures\", version = \"1\" }\n",
    );
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "examples/Cargo.toml", "[dependencies.intent]\nversion = \"1\"\n");
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "Cargo.toml", "[target.'cfg(unix)'.dependencies]\ntypescript = \"1\"\n");
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "Cargo.toml",
        "[workspace.dependencies]\nomnia = { package = \"omnia\", path = \"../specify-adapters/targets/omnia\" }\n",
    );
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "Cargo.toml",
        "[workspace.dependencies]\nomnia = \"0.35.0\"\nslice = { path = \"crates/slice\" }\n",
    );
    assert!(findings(dir.path()).is_empty());

    // The in-tree mock crate (the mock adapter core's home) is an
    // ordinary workspace member; its name collides with nothing in
    // specify-adapters and needs no allowance.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "Cargo.toml",
        "[workspace.dependencies]\nmock = { path = \"crates/mock\" }\n",
    );
    assert!(findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "crates/slice/Cargo.toml",
        "[dependencies]\nadapter = { path = \"../../specify-adapters/crates/adapter\" }\n",
    );
    assert!(!findings(dir.path()).is_empty());

    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "crates/slice/src/lib.rs", "use captures::operations;\n");
    assert!(findings(dir.path()).is_empty());
}
